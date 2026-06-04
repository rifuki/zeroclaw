import asyncio
import json
import os
import tempfile
from typing import Any

import httpx
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel


HF_BASE_URL = os.getenv("MIKUTTS_HF_BASE_URL", "https://john6666-mikutts.hf.space")
HF_MODEL = os.getenv("MIKUTTS_MODEL", "1a_miku_default_rvc_(aple)")
HF_VOICE = os.getenv("MIKUTTS_VOICE", "id-ID-GadisNeural-Female")
HF_F0_UP_KEY = float(os.getenv("MIKUTTS_F0_UP_KEY", "4"))
HF_F0_METHOD = os.getenv("MIKUTTS_F0_METHOD", "pm")
HF_INDEX_RATE = float(os.getenv("MIKUTTS_INDEX_RATE", "1"))
HF_PROTECT = float(os.getenv("MIKUTTS_PROTECT", "0.33"))
HF_TIMEOUT_SECS = float(os.getenv("MIKUTTS_TIMEOUT_SECS", "240"))
MAX_CHARS = int(os.getenv("MIKUTTS_MAX_CHARS", "800"))
ENABLE_EDGE_FALLBACK = os.getenv("MIKUTTS_EDGE_FALLBACK", "true").lower() == "true"


class SpeechRequest(BaseModel):
    model: str | None = None
    input: str | None = None
    text: str | None = None
    voice: str | None = None
    speed: float | None = None


app = FastAPI(title="Private MikuTTS bridge", version="0.1.0")


def edge_voice_name(voice: str) -> str:
    for suffix in ("-Female", "-Male"):
        if voice.endswith(suffix):
            return voice[: -len(suffix)]
    return voice


async def run_bytes(cmd: list[str], data: bytes | None = None, timeout: float = 120) -> bytes:
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdin=asyncio.subprocess.PIPE if data is not None else None,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await asyncio.wait_for(proc.communicate(data), timeout=timeout)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.wait()
        raise RuntimeError(f"command timed out: {cmd[0]}")
    if proc.returncode != 0:
        raise RuntimeError(stderr.decode("utf-8", "replace").strip() or f"{cmd[0]} failed")
    return stdout


async def transcode_to_ogg_opus(audio: bytes) -> bytes:
    return await run_bytes(
        [
            "ffmpeg",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            "pipe:0",
            "-f",
            "ogg",
            "-acodec",
            "libopus",
            "-b:a",
            "32k",
            "-vbr",
            "on",
            "pipe:1",
        ],
        data=audio,
        timeout=90,
    )


async def synthesize_edge_fallback(text: str, voice: str) -> bytes:
    with tempfile.NamedTemporaryFile(suffix=".mp3") as out:
        await run_bytes(
            [
                "edge-tts",
                "--voice",
                edge_voice_name(voice),
                "--text",
                text,
                "--write-media",
                out.name,
            ],
            timeout=90,
        )
        out.seek(0)
        return await transcode_to_ogg_opus(out.read())


def parse_sse_complete(payload: str) -> list[Any]:
    last_event = None
    for raw_line in payload.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        if line.startswith("event:"):
            last_event = line.removeprefix("event:").strip()
            continue
        if line.startswith("data:") and last_event == "complete":
            return json.loads(line.removeprefix("data:").strip())
        if line.startswith("data:") and last_event == "error":
            raise RuntimeError(line.removeprefix("data:").strip())
    raise RuntimeError("MikuTTS Space did not return a complete event")


async def synthesize_hf_miku(text: str, voice: str, speed: float) -> tuple[bytes, str]:
    payload = {
        "data": [
            HF_MODEL,
            speed,
            0,
            0,
            text,
            voice,
            HF_F0_UP_KEY,
            HF_F0_METHOD,
            HF_INDEX_RATE,
            HF_PROTECT,
        ]
    }
    timeout = httpx.Timeout(HF_TIMEOUT_SECS, connect=30)
    async with httpx.AsyncClient(timeout=timeout, follow_redirects=True) as client:
        start = await client.post(f"{HF_BASE_URL}/gradio_api/call/tts", json=payload)
        start.raise_for_status()
        event_id = start.json().get("event_id")
        if not event_id:
            raise RuntimeError("MikuTTS Space did not return event_id")

        stream = await client.get(f"{HF_BASE_URL}/gradio_api/call/tts/{event_id}")
        stream.raise_for_status()
        result = parse_sse_complete(stream.text)
        if len(result) < 3 or not isinstance(result[2], dict):
            raise RuntimeError(f"unexpected MikuTTS result: {result!r}")
        audio_url = result[2].get("url")
        info = str(result[0])
        if not audio_url:
            raise RuntimeError(f"MikuTTS returned no result audio: {result!r}")

        audio = await client.get(audio_url)
        audio.raise_for_status()
        return await transcode_to_ogg_opus(audio.content), info


@app.get("/health")
async def health() -> dict[str, Any]:
    return {
        "ok": True,
        "hf_base_url": HF_BASE_URL,
        "model": HF_MODEL,
        "voice": HF_VOICE,
        "f0_method": HF_F0_METHOD,
        "edge_fallback": ENABLE_EDGE_FALLBACK,
    }


@app.post("/v1/audio/speech")
async def speech(req: SpeechRequest) -> Response:
    text = (req.input or req.text or "").strip()
    if not text:
        raise HTTPException(status_code=400, detail={"error": {"message": "input is required"}})
    if len(text) > MAX_CHARS:
        raise HTTPException(
            status_code=400,
            detail={"error": {"message": f"input too long: {len(text)} > {MAX_CHARS} chars"}},
        )

    voice = (req.voice or HF_VOICE).strip() or HF_VOICE
    speed = req.speed if req.speed is not None else 0
    try:
        audio, info = await synthesize_hf_miku(text, voice, speed)
        return Response(content=audio, media_type="audio/ogg", headers={"X-MikuTTS-Info": info[:800]})
    except Exception as exc:
        if not ENABLE_EDGE_FALLBACK:
            raise HTTPException(status_code=502, detail={"error": {"message": str(exc)}}) from exc
        try:
            audio = await synthesize_edge_fallback(text, voice)
            return Response(
                content=audio,
                media_type="audio/ogg",
                headers={"X-MikuTTS-Fallback": "edge", "X-MikuTTS-Error": str(exc)[:800]},
            )
        except Exception as fallback_exc:
            raise HTTPException(
                status_code=502,
                detail={"error": {"message": f"MikuTTS failed: {exc}; edge fallback failed: {fallback_exc}"}},
            ) from fallback_exc
