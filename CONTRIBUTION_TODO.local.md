# ZeroClaw Local Contribution Plan

Local-only working note for Rifuki/Codex. Do not include this file in upstream PRs.

Last updated: 2026-06-04
Current validation branch: `rifuki/v0.7.5-passive-group-context-test`
Base hotfix branch: `rifuki/v0.7.5-hotfix`

Private TTS/Miku experiments are excluded from every contribution branch and
must not be included in upstream issues or PRs.

Upstream PR branch naming rule:

- Do not use `codex/` for public upstream PR branches.
- Branch names are not the same as PR titles. Match the contributor branch pattern already accepted in upstream:
  - `fix/<short-kebab-topic>` for bug fixes without a pre-existing issue.
  - `fix/<issue-number>-<short-kebab-topic>` when an upstream issue already exists.
  - `feat/<short-kebab-topic>` or `feat/<issue-number>-<short-kebab-topic>` for feature/design work.
- Recent accepted examples:
  - `fix/telegram-voice-transcription-alias`
  - `fix/media-pipeline-image-data`
  - `fix/whatsapp-lid-mentions`
  - `fix/7022-kimi-k2-temperature`
  - `feat/tts-opus-transcode`
- Preferred branches for our future upstream PRs:
  - `fix/inline-vision-history`
  - `fix/webp-vision-normalization`
  - `fix/custom-provider-vision-routing`
  - `fix/whatsapp-sticker-reply-gating`
  - `fix/whatsapp-read-receipts`
  - `fix/typing-before-prep`
  - `fix/whatsapp-quoted-media`
  - `fix/telegram-draft-update-throttle`
- Keep `codex/...` branches local/fork-only for combined testing and deployment confidence.

Upstream PR title rule:

- PR titles use Conventional Commits, as shown in the GitHub PR list:
  - `fix(channels): scope channel runtime context to the agent workspace`
  - `fix(ui/channels): avoid UTF-8 char-boundary panics in text truncation`
  - `fix(mcp): surface tool-execution errors (result.isError) instead of swallowing them`
  - `fix(runtime): guard trim_history against orphan-cascade emptying all messages`
  - `docs(providers): correct Codex-subscription credential source in catalog`
- Use `fix(<scope>): <plain-language behavior>` for our bug PRs.
- Keep the title specific enough for maintainers to understand the behavior from the PR list.

## Summary

Current contribution buckets: 12

1. WhatsApp Web LID bot mentions - merged upstream.
2. Media pipeline inline image data - merged upstream.
3. Channel history stores inline vision payloads - local hotfix, strongest next PR candidate.
4. WebP sticker vision normalization - local hotfix, small candidate issue + PR.
5. OpenAI-compatible custom provider vision routing/capability mismatch - local hotfix, medium candidate PR.
6. WhatsApp Web sticker replies to the bot - local hotfix, candidate issue + small PR.
7. WhatsApp Web read receipts / silent accepted messages - local hotfix, candidate issue + small PR.
8. Channel typing indicator delayed by slow pre-LLM preparation - local hotfix, candidate issue + small PR.
9. WhatsApp Web quoted media / file persistence - local hotfix, larger candidate issue + PR.
10. WhatsApp group shared session/history scope - local behavior exists, but needs design discussion before PR.
11. Telegram partial streaming can flood draft edits and delete interrupted drafts - production finding, candidate issue + small PR.
12. WhatsApp passive group context - private validation branch, needs design/privacy discussion before PR.

## Already Merged Upstream

### 1. WhatsApp Web LID Bot Mentions

- Upstream issue: `#7032` - closed.
- Upstream PR: `#7034` - merged.
- Scope: `mention_only` should match bot mentions addressed to the bot LID JID, not only phone JID.
- Local related commits:
  - `b42941052` - robust support for WhatsApp LID JID bot mentions.
  - `f6042df68` - resolve bot identity at startup and strip JID device-index suffix.
  - `892371ae4` - allow reply-to-bot messages to be treated as mentions.
- Status: Done. No new PR needed unless regressions appear on latest master.

### 2. Media Pipeline Inline Image Data

- Upstream issue: `#7033` - closed.
- Upstream PR: `#7035` - merged.
- Scope: image media pipeline must append actual `[IMAGE:data:<mime>;base64,...]` payload for vision providers, not just a text placeholder.
- Local related commit:
  - `c3e69366c` - embed raw image base64 data as multimodal `[IMAGE:]` marker.
- Status: Done. No new PR needed unless upstream changed behavior again.

### Telegram Transcription Provider Alias

- Upstream PR: `#7000` - merged.
- Scope: Telegram inbound voice notes wire the configured `transcription_provider` alias.
- Status: Done upstream. Removed from active contribution TODO; no duplicate PR.

## Ready For New Small Issue + PR

### 3. WhatsApp Web Read Receipts For Accepted/Gated Messages

- Problem: WhatsApp messages could still produce push notifications / unread state even after the bot received and accepted them.
- Expected behavior: once an inbound message is accepted by channel policy, the channel should mark it read, including messages later dropped by mention/content gating.
- Local related commits:
  - `8782b4ac9` - mark accepted inbound messages read.
  - `fd4895d9d` - mark gated inbound messages read.
- Suggested issue title:
  - `[Bug]: WhatsApp Web accepted inbound messages can remain unread and trigger push notifications`
- Suggested PR title:
  - `fix(channels/whatsapp-web): mark accepted inbound messages read`
- Scope boundary:
  - Do not include LID mention fixes.
  - Do not include typing indicator changes.
  - Do not change access-control decisions.
- Validation needed before filing:
  - Re-check latest `upstream/master` still lacks equivalent read receipt behavior.
  - Use anonymized logs only; no real message text, phone numbers, or chat IDs.

### 4. Channel Typing Indicator Before Slow Pre-LLM Preparation

- Problem: direct or mentioned channel messages can be received immediately, but typing/thinking feedback is delayed until after memory recall/context compression.
- Observed before fix:
  - Message received and marked read immediately.
  - `Proactive context compression applied before LLM call`.
  - `Starting LLM call elapsed_before_llm_ms=5685`.
  - Typing indicator appears only around LLM start.
- Expected behavior: once a message is accepted and the channel will reply, typing feedback should start before slow pre-LLM preparation.
- Local related commit:
  - `486217d36` - start typing before slow preparation.
- Live VPS validation:
  - Direct chat: `start typing` appeared about 3 ms after message receive.
  - Group message with compression: `start typing` appeared about 2 ms after message receive, while LLM start still waited about 5.6 s.
- Suggested issue title:
  - `[Bug]: Channel typing indicator is delayed until after slow context compression`
- Suggested PR title:
  - `fix(channels): start typing before slow pre-LLM preparation`
- Scope boundary:
  - Do not optimize LLM latency in this PR.
  - Do not change context compression behavior.
  - Do not change WhatsApp mention gating.
  - Keep no-reply precheck behavior safe; avoid typing for messages classified as `NO_REPLY`.
- Validation already run locally:
  - `cargo fmt --all -- --check`
  - `cargo test -p zeroclaw-channels process_channel_message_ --lib`
- Validation needed for upstream PR:
  - Port patch onto `upstream/master` because master orchestrator differs from v0.7.5.
  - Re-run targeted tests on master.

### 7. WhatsApp Web Sticker Replies To The Bot

- Problem: WhatsApp Web sticker-only messages arrive with empty text and no downloaded attachment, so they are ignored before reply-to-bot / structured mention gating can run.
- Observed in VPS logs:
  - `content_len=0, attachments=0`
  - `WhatsApp Web: ignoring empty or non-text message`
- Expected behavior: when a sticker message is sent in a DM, mentions the bot, or replies to the bot in a group, the channel should pass a small placeholder such as `[Sticker]` to the agent so it can respond.
- Local related patch:
  - Adds sticker detection via `sticker_message`.
  - Reuses existing `is_reply_to_bot` / `extract_quoted_participant` support for sticker `context_info`.
  - Converts empty sticker-only content to `[Sticker]` before the empty-message guard.
- Suggested issue title:
  - `[Bug]: WhatsApp Web ignores sticker replies before reply-to-bot gating can run`
- Suggested PR title:
  - `fix(channels/whatsapp-web): allow sticker replies to reach reply gating`
- Scope boundary:
  - Do not add full sticker download/vision support in this PR.
  - Do not change mention-only policy for unrelated group chatter.
  - Do not mix with read receipts or group session-scope changes.
- Validation already run locally:
  - `cargo fmt --all -- --check`
  - `cargo test -p zeroclaw-channels --features whatsapp-web sticker --lib`
  - `cargo test -p zeroclaw-channels --features whatsapp-web is_reply_to_bot_matches_sticker_context --lib`
  - `cargo test -p zeroclaw-channels --features whatsapp-web contains_bot_mention --lib`
- Validation needed before upstream PR:
  - Port patch onto `upstream/master`.
  - Test in a real WhatsApp group by replying to a bot message with a sticker.

### 8. WhatsApp Web Quoted Media / File Persistence

- Problem: replying to a prior WhatsApp file/image/sticker with text leaves the actual media inside `context_info.quoted_message`, while the channel only downloaded top-level media from the current message.
- Related symptom:
  - A direct zip attachment can show `attachments=1`, but unknown document/file attachments were skipped by the media pipeline, so the agent may still behave as if no usable file exists.
  - A text reply to a file can show `attachments=0` because the file is quoted media, not top-level media.
- Expected behavior:
  - Download media from both the current message and the quoted message.
  - Support sticker download as image/webp so vision-capable providers can inspect the sticker image.
  - Persist inbound WhatsApp media into `{workspace}/whatsapp_files/` and annotate non-image files with the saved path, byte size, and MIME type.
- Local related patch:
  - Adds `WhatsAppWebChannel::with_workspace_dir` and wires it from orchestrator config.
  - Extracts `context_info.quoted_message`.
  - Downloads image/video/document/sticker from top-level and quoted WhatsApp messages.
  - Persists downloaded files under `whatsapp_files/`.
  - Adds media pipeline annotation for `MediaKind::Unknown` files such as `.zip`.
- Suggested issue title:
  - `[Bug]: WhatsApp Web drops quoted media and does not expose document attachments to the agent`
- Suggested PR title:
  - `fix(channels/whatsapp-web): persist quoted media and document attachments`
- Scope boundary:
  - Do not add archive extraction logic in the channel.
  - Do not change tool security or workspace policy.
  - Do not mix with passive group observation.
- Validation already run locally:
  - `cargo fmt --all -- --check`
  - `cargo test -p zeroclaw-channels --features whatsapp-web write_inbound_attachment_persists_inside_workspace --lib`
  - `cargo test -p zeroclaw-channels --features whatsapp-web extract_quoted_message_reads_extended_text_context --lib`
  - `cargo test -p zeroclaw-channels --features whatsapp-web file_annotation_includes_saved_path_and_size --lib`
  - `cargo test -p zeroclaw-channels --features whatsapp-web media_pipeline --lib`
  - `cargo test -p zeroclaw-channels --features whatsapp-web whatsapp_web --lib`
  - `cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings`
- Validation needed before upstream PR:
  - Test real WhatsApp: direct zip/file, reply to prior zip/file, and sticker message.
  - Port onto `upstream/master`.

### 9. Custom Provider Vision Capability / Dedicated Vision Routing

- Problem: inbound WhatsApp stickers/quoted images are now downloaded and persisted, but the active `custom:https://...` provider profile is treated as vision-capable even when the selected concrete model rejects image input.
- Observed in VPS logs after deploy:
  - WhatsApp messages with current/quoted stickers produced `attachments=1`.
  - Files were persisted under `~/.zeroclaw/workspace/whatsapp_files/` as `sticker.webp` / `quoted-sticker.webp`.
  - The LLM call then failed with provider error `No endpoints found that support image input` for the active text model.
- Root-cause candidates:
  - The `custom:` OpenAI-compatible provider factory hardcodes `supports_vision = true`.
  - Runtime `multimodal.vision_provider` routing is only used when the default provider reports `!supports_vision()`, so a false-positive provider capability prevents dedicated vision routing.
  - `vision_provider` creation may not inherit decrypted config credentials for an encrypted custom provider profile.
- Local related commits:
  - `72bed18ad` - allow disabling vision for custom OpenAI-compatible endpoints.
  - `7f8b9067f` - route inbound media turns to configured vision provider/model.
- Live VPS config under test:
  - Default custom Mimo profile keeps `vision = false`.
  - `[multimodal] vision_provider = "custom:https://9router.rifuki.dev/v1"`.
  - `[multimodal] vision_model = "mimo-custom/mimo-v2.5"`.
  - Main model remains `mimo-custom/mimo-v2.5-pro`.
- MiMo capability finding, 2026-06-04:
  - Xiaomi's official API documentation lists image understanding support for
    `mimo-v2.5` and `mimo-v2-omni`, not `mimo-v2.5-pro`.
  - `mimo-v2.5-pro` returning `No endpoints found that support image input` is
    therefore expected upstream behavior, not evidence that 9router corrupted a
    valid Pro image request.
  - For a MiMo-only deployment, use Pro for text/agent turns and `mimo-v2.5` for
    vision turns, or use `mimo-v2.5` for both if exact 1:1 routing is required.
  - 9router still has a capability-discovery gap: its current
    `/v1/models/image-to-text` response does not advertise MiMo models, and both
    the local fork and latest upstream model metadata leave Xiaomi MiMo without
    `imageToText` discovery metadata.
  - Direct VPS validation through ZeroClaw and the same 9router OpenAI-compatible
    endpoint succeeded: `mimo-custom/mimo-v2.5` received a generated red PNG and
    answered `Merah`.
- Suggested issue title:
  - `[Bug]: Custom OpenAI-compatible providers can over-advertise vision and bypass configured vision routing`
- Suggested PR title:
  - `fix(providers): respect explicit vision capability for custom providers`
- Scope boundary:
  - Do not mix with WhatsApp media download/persistence.
  - Do not change model fallback or retry policy.
  - Do not commit real endpoint credentials or message content in tests/logs.
- Validation needed before upstream PR:
  - Decide whether the minimal fix is a per-provider config flag, unconditional `vision_provider` routing when configured, credential inheritance for configured vision providers, or a combination.
  - Add unit tests around `custom:` provider capability and `multimodal.vision_provider` precedence.
  - Re-test a text-only custom model with an image marker: it should fail fast with a capability error or route to the configured vision provider, not send image input to a text-only endpoint.

### 10. Channel History Inline Vision Payload Persistence

- Problem: successful image/sticker turns append `[IMAGE:data:<mime>;base64,...]` to the current user content, then persist that full payload into channel history/session storage.
- Observed after switching VPS `[multimodal].vision_model` to `cx/gpt-5.5`:
  - Vision itself works; the model described WhatsApp sticker/image content successfully.
  - Slow turns were dominated by pre-LLM context work, not the provider call.
  - Example log pattern: `Proactive context compression applied before LLM call`, then `Starting LLM call elapsed_before_llm_ms` around 57-71 seconds, while the actual `llm_call_ms` was only around 2.6-3.9 seconds.
- Root cause:
  - `process_channel_message` persisted `msg.content` after media enrichment.

### 11. Telegram Draft Streaming Flood / Interrupted Draft Deletion

- Problem: Telegram `stream_mode = "partial"` can become visibly slow when `draft_update_interval_ms = 0`, because every streamed text delta triggers a synchronous `editMessageText` call.
- Related symptom:
  - The user sees the bot message being rewritten slowly, token/chunk by token/chunk.
  - If the user sends a newer prompt while the older draft is still streaming, `interrupt_on_new_message = true` cancels the old request and `cancel_draft()` deletes the visible draft message, so the in-progress answer appears to disappear.
- Observed on VPS:
  - `[channels.telegram] stream_mode = "partial"`
  - `draft_update_interval_ms = 0`
  - `interrupt_on_new_message = true`
  - Logs showed `Interrupting previous in-flight request for sender channel=telegram` followed by `Cancelled in-flight channel request due to newer message`.
- Production mitigation applied on 2026-06-03:
  - Set Telegram `stream_mode = "off"`.
  - Set `draft_update_interval_ms = 1000`.
  - Restarted `zeroclaw.service`.
- Expected upstream behavior:
  - Config should reject or clamp `draft_update_interval_ms = 0` for partial streaming, or document that zero disables throttling and can flood platform edit APIs.
  - Optional design discussion: interrupted Telegram drafts could be finalized with a short cancellation marker instead of being silently deleted.
- Suggested issue title:
  - `[Bug]: Telegram partial streaming can flood editMessageText when draft_update_interval_ms is zero`
- Suggested PR title:
  - `fix(channels/telegram): clamp partial draft update interval`
- Scope boundary:
  - Do not change provider streaming behavior.
  - Do not change interrupt-on-new-message semantics in the same PR unless maintainers explicitly want that design.
  - Do not include real chat text, usernames, message IDs, or bot tokens in issue/PR logs.
- Validation needed before upstream PR:
  - Add unit test for `with_streaming(StreamMode::Partial, 0)` or config normalization, depending on chosen implementation.
  - Add unit test proving draft updates are throttled to a sane minimum.
  - Re-run Telegram/channel targeted tests and clippy on fresh `upstream/master`.
  - That enriched content includes inline base64 image data.
  - Later turns reload the payload before provider-layer multimodal trimming can run, so channel-level compression pays the full base64 cost.
- Hermes/OpenClaw comparison note:
  - Hermes keeps media as local/cache/tool artifacts and passes multimodal payloads for the active turn, rather than letting raw image payloads remain as ordinary long-term conversation text.
- Local related patch:
  - Persist a compact history version of user media turns by stripping loadable `[IMAGE:data:...]` markers.
  - Restore the full current-turn image payload only inside the provider request.
  - Strip legacy inline image payloads from older restored history before channel-level context compression.
  - Store autosave memory from the compact version, not the inline base64 payload.
  - Roll back failed current turns using the compact persisted content.
- Suggested issue title:
  - `[Bug]: Channel history persists inline image payloads and makes later turns slow`
- Suggested PR title:
  - `fix(channels): avoid persisting inline vision payloads in history`
- Scope boundary:
  - Do not change media download/persistence.
  - Do not change provider image support.
  - Do not change context compression policy beyond removing raw media payloads from channel history.
  - Do not include real message content or media data in logs/tests.
- Validation already run locally:
  - `cargo test -p zeroclaw-channels --features whatsapp-web --lib image_payload` - passed.
  - `cargo test -p zeroclaw-channels --features whatsapp-web --lib channel_history_content_for` - passed.
  - `cargo test -p zeroclaw-channels --features whatsapp-web --lib e2e_failed_vision_turn_does_not_poison_follow_up_text_turn` - passed.
  - `cargo fmt --all -- --check` - passed.
  - `cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings` - passed.
- CI/CD and VPS:
  - Commit: `9fa09bd46` - `fix(channels): avoid persisting inline vision payloads`.
  - GitHub Actions run: `26845180965` - passed.
  - Artifact SHA target: `9fa09bd461fd09a88cc26551b02f48ecb41853ae`.
  - VPS host: `tencent-rifuki`.
  - Deployed build SHA: `9fa09bd461fd09a88cc26551b02f48ecb41853ae`.
  - Service status after restart: `active/running`.
  - Backup created: `/home/rifuki/.local/bin/zeroclaw.bak.20260603041930`.
  - Live config: `[multimodal].vision_model = "cx/gpt-5.5"`.
  - Startup smoke test: WhatsApp Web connected successfully, Telegram listening.
- Validation needed before upstream PR:
  - Port onto `upstream/master`.
  - Re-test real WhatsApp sticker/image: current turn should still see image; next turn should not spend tens of seconds compressing prior base64.
- Port status:
  - 2026-06-03: direct cherry-pick of `9fa09bd46` onto `upstream/master` conflicts in `crates/zeroclaw-channels/src/orchestrator/mod.rs`.
  - Reason: upstream master has newer channel timestamp/history flow (`timestamp_channel_user_content`, `process_channel_message_body`) that must be preserved while adding compact media-history storage.
  - Best action: wait for `#7040` if avoiding conflict churn, or do a manual port branch and open as draft.

### WebP Sticker Vision Normalization

- Problem: WhatsApp stickers are commonly `image/webp`, but some vision providers reject WebP even though they accept PNG/JPEG.
- Observed in VPS logs:
  - Sticker media reached the media pipeline.
  - Vision call failed with provider error indicating the image media type was unsupported.
- Expected behavior: WebP image attachments should be normalized to PNG before the `[IMAGE:data:...]` marker is emitted when vision is active.
- Local related commit:
  - `529944a53` - normalize WebP images for vision.
- Suggested issue title:
  - `[Bug]: WhatsApp WebP stickers can fail vision providers that do not accept image/webp`
- Suggested PR title:
  - `fix(channels): normalize webp images for vision`
- Scope boundary:
  - Do not change WhatsApp message gating.
  - Do not change provider routing.
  - Do not add sticker description caching in this PR.
  - Keep fallback behavior if WebP decode fails.
- Validation already run locally:
  - `cargo test -p zeroclaw-channels --features whatsapp-web media_pipeline --lib` - passed.
  - `cargo fmt --all -- --check` - passed.
  - `cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings` - passed.
- Validation needed before upstream PR:
  - Port onto `upstream/master`.
  - Confirm dependency/features are acceptable for upstream bundle size.

## Discuss Before PR

### 5. WhatsApp Group Shared Session / History Scope

- Problem/question: group chats may behave like each participant has an individual session, while the desired behavior may be shared group context.
- Current live finding:
  - Current `upstream/master` / WebP PR branch still computes WhatsApp group history as `reply_target + sender`, so visible group history is per-sender unless a separate local branch changes it.
  - Unmentioned pass-through group chatter is still dropped by mention/pattern gating and is not written to history.
  - If the desired behavior is "listen silently to non-mentioned group chatter for context", that needs an explicit observe/history config because it changes privacy and attention semantics.
- Live config-only experiment, 2026-06-04:
  - The deployed v0.7.5 hotfix already has shared WhatsApp group history from `ca094bf31`.
  - Setting `channels.whatsapp.group_mention_patterns = []` allowed unmentioned group messages to reach the orchestrator instead of being dropped.
  - Setting `agent.precheck.enabled = true` and `agent.precheck.model = "mimo-custom/mimo-v2.5-pro"` let Mimo classify an unmentioned group message as `NO_REPLY[INFO]`, so the message entered history without a visible bot reply.
  - The first successful no-reply decision took about 30 seconds total even though the Mimo precheck call itself took about 6 seconds, because memory/history preparation runs before precheck.
  - Later turns exposed an existing inline image payload in shared history. Mimo rejected the image input, precheck failed open to `REPLY`, and the main turn also failed. This confirms passive observation must be tested together with the inline vision history fix.
  - Config was restored after the experiment. Re-test on `rifuki/v0.7.5-channel-fixes-test`, which includes `9fa09bd46`.
- Current conclusion:
  - Shared session scope and passive observation are separate concerns.
  - Existing config can approximate passive observation, but precheck latency and fail-open behavior make it unsuitable as a silent-context guarantee without further design discussion.
- Production evidence, 2026-06-03:
  - A group message containing a link was logged, then dropped with `message from ... did not match mention patterns, dropping`.
  - A follow-up group message tagging another user with context was logged, then dropped by the same mention-pattern gate.
  - The later bot-addressed message reached the agent only as `gimana menurutmu?`, with the bot mention stripped and without the previous link/context.
  - Result: the assistant answered from stale prior conversation context instead of the immediately preceding group chatter.
- Local related commit:
  - `ca094bf31` - parse media captions, allow structural mentions to bypass regex gating, and enable shared group chat history for WhatsApp groups.
- Why not PR immediately:
  - This is product/design behavior, not just an obvious defect.
  - Some users may want per-sender memory in groups for privacy or personalization.
  - Better shape may be config-driven, e.g. `session_scope = "group" | "sender" | "thread"` per channel/group.
- Best next step:
  - Open a design/feature issue first, not a PR.
  - Compare with OpenClaw/Hermes behavior and propose a flexible config.
- Suggested issue title:
  - `[Feature]: Configurable session scope for channel group chats`
- Suggested PR title later:
  - `feat(channels): add configurable group session scope`
- Scope boundary:
  - Do not mix with read receipts.
  - Do not mix with typing indicator.
  - Do not silently change all group chats without a migration/config story.

### 12. WhatsApp Passive Group Context

- Problem/question: when WhatsApp group reply gating is enabled, unmentioned group
  messages are dropped before they can become context for a later addressed
  question.
- Desired behavior under test:
  - Store accepted but unaddressed WhatsApp group messages in shared group history.
  - Do not start an agent turn, call a provider, show typing, run tools, or send a
    reply for those passive messages.
  - Let a later message that addresses the bot use the stored context.
- Source-of-truth split:
  - `WhatsAppConfig.passive_group_context` opts into the behavior.
  - The WhatsApp adapter determines whether a message was addressed by mention,
    pattern, or reply.
  - The orchestrator/session history remains the only history store.
- Local private validation branch:
  - `rifuki/v0.7.5-passive-group-context-test`
  - Commit: `5e8814282` - `feat(channels/whatsapp-web): store passive group context`.
- Local validation already run:
  - `cargo fmt --all -- --check`
  - `cargo test -p zeroclaw-config`
  - `cargo check -p zeroclaw-channels`
  - `cargo test -p zeroclaw-channels --features 'whatsapp-web,channel-telegram' --lib`
  - `cargo clippy -p zeroclaw-channels --features 'whatsapp-web,channel-telegram' --lib -- -D warnings`
  - `git diff --check`
- CI/CD and VPS:
  - GitHub Actions run: `26938745866` - passed.
  - Artifact SHA target: `5e8814282fa09a34e9463ec95f475a1654c91e83`.
  - Deployed to `tencent-rifuki` on 2026-06-04.
  - Service status after restart: `active`.
  - Live model remains `mimo-custom/mimo-v2.5-pro`.
  - `[agent.precheck].enabled = false` and `[tts].enabled = false`.
  - `[channels.whatsapp].passive_group_context = true`.
- Live test pending:
  - Send an unmentioned group fact. Doloris must remain silent and no provider call
    should occur.
  - Then mention Doloris and ask for the fact. Only this addressed turn may call
    Mimo and should answer from the stored group context.
- Why not PR immediately:
  - Passive observation changes privacy and attention semantics.
  - It is useful only when the chosen group session scope allows later participants
    to see the same context.
  - Upstream should discuss whether this belongs under WhatsApp config, generic
    channel group config, or a broader session/observation policy.
- Scope boundary:
  - Do not mix with configurable group session scope.
  - Do not use LLM precheck as the silent-context guarantee.
  - Do not call any model for passive messages.
  - Do not mix with TTS, read receipts, typing indicators, or media persistence.

## Suggested Submission Order

1. Channel history inline vision payload persistence.
   - Suggested PR: `fix(channels): avoid persisting inline vision payloads in history`
   - Why first: highest confidence, already validated in CI/VPS, fixes real slow-response behavior, and is a clean follow-up to merged media-pipeline vision support.
   - Caveat: touches `orchestrator/mod.rs`; if upstream PR `#7040` is still open, wait for it to merge or expect rebase conflict.
2. WebP sticker vision normalization.
   - Suggested PR: `fix(channels): normalize webp images for vision`
   - Why early: small, focused, easy to explain, and fixes WhatsApp sticker images for providers that reject WebP.
   - Caveat: depends on the upstream image-payload behavior from `#7035`, which is already merged.
3. Custom provider vision capability / dedicated vision routing.
   - Suggested PR: `fix(providers): respect explicit vision capability for custom providers`
   - Why next: important for custom/OpenAI-compatible deployments, but touches provider config/routing behavior and needs careful tests.
4. WhatsApp Web sticker replies to the bot.
   - Suggested PR: `fix(channels/whatsapp-web): allow sticker replies to reach reply gating`
   - Why here: small WhatsApp-specific bug, but less universal than history/pipeline fixes.
5. WhatsApp Web read receipts / silent accepted messages.
   - Suggested PR: `fix(channels/whatsapp-web): mark accepted inbound messages read`
   - Why here: useful UX fix, but read/unread semantics can be maintainer-preference sensitive.
6. Channel typing indicator before slow pre-LLM preparation.
   - Suggested PR: `fix(channels): start typing before slow pre-LLM preparation`
   - Why later: important UX, but precheck/no-reply timing and `orchestrator/mod.rs` overlap make it more review-sensitive.
7. WhatsApp Web quoted media / file persistence.
   - Suggested PR: `fix(channels/whatsapp-web): persist quoted media and document attachments`
   - Why later: very useful, but larger blast radius because it adds media download, workspace persistence, file annotations, and quoted-message handling.
8. WhatsApp group shared session scope / passive group observation.
   - Suggested issue first: `[Feature]: Configurable session scope for channel group chats`
   - Why last: this is product/design/privacy behavior, not a narrow bug. Start with discussion, not PR.

Reasoning:

- Prefer one concern per PR and avoid one giant upstream diff from the combined local branch.
- If `#7040` is still open, hold orchestrator-heavy PRs and use WebP normalization as the least-conflicting fallback first PR.
- Group session / passive observation behavior needs maintainer agreement before code.
- Already merged items should stay closed unless upstream regresses.

## Open PR Overlap Audit

Last checked: 2026-06-03.

Command source: `gh pr list --repo zeroclaw-labs/zeroclaw --state open --limit 500`.

Open PR count at audit time: 164.

Exact overlap with our pending PR candidates:

- Inline vision payload history cleanup: no exact open PR found.
- WebP sticker vision normalization: no exact open PR found.
- WhatsApp sticker reply gating: no exact open PR found.
- WhatsApp read receipts / silent accepted messages: no exact open PR found.
- Channel typing before slow pre-LLM preparation: no exact open PR found.
- WhatsApp quoted media / file persistence: no exact open PR found.
- Group shared session scope / passive group observation: no exact open PR found.
- Custom provider vision routing/capability mismatch: partial/near overlap exists; see `#5892`.

Near-overlap / conflict-risk PRs:

- `#7040` - `fix(channel): restore WhatsApp interrupt_on_new_message`
  - Touches `crates/zeroclaw-channels/src/orchestrator/mod.rs`.
  - Not the same feature, but conflicts with orchestrator-heavy PRs.
- `#7066` - `fix(channels): excise default-model-provider credential fallback`
  - Touches provider routing in `orchestrator/mod.rs`.
  - Not the same as inline history, but can conflict with custom vision routing.
- `#7121` - `fix(channels): scope channel runtime context to the agent workspace`
  - Touches `orchestrator/mod.rs`.
  - Not our feature, but another conflict-risk patch.
- `#7017` - `refactor(channels/whatsapp_web): share allowlist matching via aspect-std (stateless)`
  - Touches `whatsapp_web.rs`.
  - Not sticker/read receipt/quoted media, but can conflict with WhatsApp Web file edits.
- `#6973` - `fix(channels/whatsapp-web): pass LID JIDs unchanged to whatsapp-rust 0.6+`
  - Touches `whatsapp_web.rs` and dependency versions.
  - LID/reply routing related, not our sticker/quoted-media work.
- `#7119` - `fix(runtime): guard trim_history against orphan-cascade emptying all messages`
  - Runtime history trim, not channel image payload history.
- `#5892` - `fix(providers,runtime): three production blockers — tool_choice, orphaned tool_use, and vision capability`
  - Draft and broad.
  - Partial overlap with our custom provider vision capability topic, but not the same as dedicated `multimodal.vision_provider` routing or inline channel-history payload cleanup.

Current recommendation after audit:

- Do not duplicate `#5892` with a broad provider-vision PR.
- Inline vision history cleanup is still a clean, unclaimed PR candidate, but it must be ported carefully because active open PRs touch `orchestrator/mod.rs`.
- WebP sticker normalization remains the lowest-conflict fallback PR because no open PR currently targets WebP vision normalization.

## PRs After #7040 Touching WhatsApp

Last checked: 2026-06-03.

Scope of this audit:

- Numeric PR range: `#7041` and newer.
- Match condition: changed files, title, labels, or branch mention `whatsapp`, `whatsapp_web`, `wati`, or `channel:whatsapp`.
- Open PR files were checked per PR via `repos/zeroclaw-labs/zeroclaw/pulls/<number>/files` after bulk GraphQL `files` query timed out.
- Merged/closed PR files were checked from `gh pr list --state merged/closed --limit 300 --json ... files`.

Findings:

- Open PRs touching WhatsApp after `#7040`:
  - `#7102` - `fix(channels-runtime): remove redundant Option wrapping and replace unwrap() with expect()`
    - State: open, `REVIEW_REQUIRED`, merge state `DIRTY`.
    - Files: `crates/zeroclaw-channels/src/telegram.rs`, `crates/zeroclaw-channels/src/whatsapp_web.rs`, `crates/zeroclaw-runtime/src/tools/mod.rs`.
    - Relevance: cleanup/diagnostic-only per PR body; not media/history/sticker/read-receipt/typing behavior.
  - `#7132` - `chore: scrub stale "zeroclaw onboard" references across docs, scripts, channels, providers, and runtime`
    - State: open, `REVIEW_REQUIRED`, merge state `BLOCKED`.
    - Files include `crates/zeroclaw-channels/src/whatsapp.rs` and `docs/book/src/channels/whatsapp.md`.
    - Relevance: stale CLI wording/docs sweep; not WhatsApp Web behavior.
- Merged PRs touching WhatsApp after `#7040`:
  - `#7050` - `feat(tts): transcode to OGG/Opus for voice notes (Telegram + WhatsApp)`
    - State: merged on 2026-06-02.
    - Files: `crates/zeroclaw-channels/src/telegram.rs`, `crates/zeroclaw-channels/src/tts.rs`, `crates/zeroclaw-channels/src/whatsapp_web.rs`.
    - Relevance: TTS/voice-note delivery, not media/history/sticker/read-receipt/typing behavior.
- Closed-unmerged PRs touching WhatsApp after `#7040`:
  - None found in the checked recent closed range.

Conclusion:

- There are WhatsApp-touching PRs after `#7040`, but none overlap our pending WhatsApp/media/history contribution buckets.
- `#7102` can conflict mechanically with `whatsapp_web.rs` patches.
- `#7132` can conflict mechanically with docs/`whatsapp.rs`, but not with WhatsApp Web media logic.
- `#7050` is already merged and may need to be included when rebasing any WhatsApp Web TTS-adjacent work, but it is unrelated to our sticker/media/history/read-receipt/typing fixes.

## All-State PR Audit Notes

Last checked: 2026-06-03 against live upstream `master` at `40be7738f3c5b18170017428e52fd11595c08cb8`.

Audit sources:

- `gh pr list --repo zeroclaw-labs/zeroclaw --state open --limit 500` with PR body/title/review metadata.
- `gh pr list --repo zeroclaw-labs/zeroclaw --state open --limit 500` with changed files.
- Targeted `gh search prs` for open, merged, and closed-unmerged matches until GitHub search API rate-limit kicked in.
- Direct `gh pr view` for the relevant merged/closed PRs found by search.
- `git grep` against `upstream/master` for current media/history behavior.

Important limitation:

- GitHub search API hit a rate limit during the long all-state keyword sweep. Open PR audit is complete from `gh pr list`; merged/closed audit is strongest for the exact/near matches listed below and for recent PRs surfaced before the rate limit.

`#7040` current status:

- `#7040` - `fix(channel): restore WhatsApp interrupt_on_new_message`
  - State: open.
  - Draft: false.
  - Review decision: `REVIEW_REQUIRED`.
  - Merge state: `BLOCKED`.
  - Reviews: none at latest check.
  - Files: `crates/zeroclaw-channels/src/orchestrator/mod.rs`, `crates/zeroclaw-config/src/schema.rs`, `docs/book/src/channels/whatsapp.md`.
  - Conclusion: not merged, not approved, and not the same feature as inline vision history cleanup. It is a conflict-risk PR because it touches `orchestrator/mod.rs`.

WhatsApp interrupt historical audit:

- `#4371` - `feat(channel): add interrupt_on_new_message support for WhatsApp`
  - State: merged on 2026-03-28.
  - Author: `@tmigone`.
  - Old paths: `src/channels/mod.rs`, `src/config/schema.rs`, `src/onboard/wizard.rs`, plus agent files.
  - `#7040` explicitly says `Supersedes #4371` because v0.8/current crate architecture lost that WhatsApp wiring.
  - Conclusion: WhatsApp interrupt was accepted historically, but current `upstream/master` still needs `#7040` or equivalent restore.
- Other accepted channel interrupt precedents:
  - `#3917` - Mattermost interrupt support, merged on 2026-03-19.
  - `#3918` - Discord interrupt support, merged on 2026-03-19.
  - `#4070` - Matrix interrupt support, merged on 2026-03-20.
  - `#964` - Telegram interrupt support, merged.
- Closed-unmerged related attempts:
  - `#3895`, `#4013`, `#4067` - Matrix interrupt variants that were replaced by merged `#4070`.
  - `#3900` - thread-aware cancellation scoping, closed unmerged.
- Current code check:
  - `upstream/master` currently reads interrupt config for Telegram, Slack, Discord, Mattermost, and Matrix.
  - It does not currently wire `channels.whatsapp.default.interrupt_on_new_message` into `InterruptOnNewMessageConfig` before `#7040`.
  - Both current WhatsApp backends emit `channel = "whatsapp"`, so `#7040` is the expected current-architecture restore path.

Merged PRs that are related context but not duplicates:

- `#7035` - `fix(channels/media-pipeline): inline image data for vision`
  - Merged and approved.
  - This is our earlier narrow fix for adding `[IMAGE:data:...]` in `media_pipeline.rs`.
  - New inline-history PR should reference this as the upstream behavior that exposed the follow-up issue: image data now reaches vision providers, but raw inline base64 must not stay in channel history/session.
- `#7008` - `fix(channels): deliver WhatsApp replies for LID JIDs and empty sanitization`
  - Merged and approved after a requested-change cycle.
  - Related to LID delivery and empty replies, not sticker gating, quoted media persistence, read receipts, or inline vision history.
- `#3734` - `fix(agent): strip vision markers from history for non-vision providers`
  - Merged in old architecture paths (`src/channels/mod.rs`, `src/multimodal.rs`).
  - Fixes non-vision provider history poisoning and rollback-store sync.
  - Not a duplicate of inline-history cleanup because current upstream still persists successful vision turns with `[IMAGE:data:...]` before LLM call; existing non-vision cleanup only runs when `!active_model_provider.supports_vision()`.
- `#4264` - `feat(multimodal): route image messages to dedicated vision provider`
  - Merged in old architecture paths.
  - Related to configured `vision_provider` routing, not channel-history payload compaction.
- `#6114` - `fix(provider): strip media markers in auxiliary LLM calls`
  - Merged and approved.
  - Sanitizes auxiliary classifier/summarizer calls, not the main channel history/session persistence path.
- `#6882` - `fix(runtime): sanitize compressor media markers before truncation`
  - Merged and approved.
  - Runtime context-compressor privacy/truncation fix, not channel session persistence.

Closed-unmerged PRs that prove prior interest but are not active blockers:

- `#3676` - `fix(channel): prevent vision error from poisoning conversation history`
  - Closed unmerged.
  - Similar concept to vision/history poisoning, but targets older paths and focused on non-vision provider errors and JSONL rollback.
  - Useful as related historical context, not a superseded active PR.
- `#3805` - `feat(whatsapp): mention_only, reply context, image handling, MCP vision fallback, and per-chat sessions`
  - Closed unmerged.
  - Very broad XL WhatsApp/Signal/provider/config/tools PR.
  - It touched reply context, image handling, and per-chat sessions, but its size/scope is exactly what we should avoid. Our future PRs should split the useful ideas into narrow fixes.

Open approved PRs:

- None of the open `APPROVED` PRs at audit time target our pending features.
- Approved unrelated/channel-adjacent examples include `#7086` Discord audio fallback and `#5796` XML tool-result stripping from channel responses.
- Several approved PRs are dirty/blocked but unrelated to WhatsApp media, read receipts, WebP normalization, typing-before-prep, or inline vision history.

Current `upstream/master` code check:

- `MediaPipeline::process_image` now emits `[IMAGE:data:<mime>;base64,...]` from `#7035`.
- `process_channel_message` still calls `timestamp_channel_user_content(&msg.content)` and then `append_sender_turn(...)` before the LLM call.
- If `msg.content` contains `[IMAGE:data:...]`, that full inline base64 is persisted to memory/session history.
- Existing sanitizers cover:
  - auxiliary reply-intent classifier prompts via `strip_media_markers`;
  - runtime context-compressor truncation;
  - older history for non-vision providers only.
- Existing sanitizers do not cover:
  - successful current vision turns being persisted as raw inline base64;
  - old successful vision turns causing later channel-level context compression to scan huge base64 blobs.

Conclusion:

- No open, merged, approved, or closed-unmerged PR was found that exactly implements `fix(channels): avoid persisting inline vision payloads in history`.
- The best wording is not "brand new unrelated bug"; it is a focused follow-up to `#7035`, with related context from `#3734`, `#6114`, and `#6882`.
- For PR body, include:
  - `Refs #7035`
  - `Related #3734`
  - `Related #6114`
  - `Related #6882`
  - Do not use `Supersedes` unless a maintainer points to a specific active PR.

## Combined Local Validation Branch

Branch: `codex/v075-combined-channel-fixes`

Purpose: prove pending local contribution areas can coexist in one deployable v0.7.5 build before splitting or porting them for upstream.

Included pending areas:

- WhatsApp group shared session/history scope.
- WhatsApp Web read receipts for accepted/gated inbound messages.
- Channel typing indicator before slow pre-LLM preparation.
- WhatsApp Web sticker replies to the bot.
- WhatsApp Web quoted media / file persistence.
- OpenAI-compatible custom provider vision capability/routing.
- WebP sticker vision normalization.
- Channel history inline vision payload cleanup.

Latest branch commit:

- `9fa09bd46` - avoid persisting inline vision payloads.

CI/CD:

- GitHub Actions run: `26845180965`
- Workflow: `Rifuki Build ZeroClaw Linux`
- Result: success.
- Build ref: `codex/v075-combined-channel-fixes`
- Build SHA: `9fa09bd461fd09a88cc26551b02f48ecb41853ae`

VPS deploy:

- Host: `tencent-rifuki`
- Deployed build SHA: `9fa09bd461fd09a88cc26551b02f48ecb41853ae`
- Service status after restart: `active/running`
- Backup created: `/home/rifuki/.local/bin/zeroclaw.bak.20260603041930`
- Startup smoke test: WhatsApp Web connected successfully, Telegram listening.

Local validation already run:

- `cargo fmt --all -- --check` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web conversation_history_key_ --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web interruption_scope_key_shares_whatsapp_group_across_senders --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web read_receipts --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web mark_inbound --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web process_channel_message_starts_typing_before_slow_recall --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web sticker --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web is_reply_to_bot_matches_sticker_context --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web contains_bot_mention --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web write_inbound_attachment_persists_inside_workspace --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web extract_quoted_message_reads_extended_text_context --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web file_annotation_includes_saved_path_and_size --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web media_pipeline --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web whatsapp_web --lib` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib image_payload` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib channel_history_content_for` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib strip_historical_image_payloads` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib e2e_failed_vision_turn_does_not_poison_follow_up_text_turn` - passed.
- `cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings` - passed.

Known unrelated local test noise:

- `cargo test -p zeroclaw-channels --features whatsapp-web --lib` reported `1004 passed, 2 failed`.
- The two failures were Telegram `build_channel_by_id_*` tests under the WhatsApp Web feature set.
- Do not mix those Telegram test failures into the three WhatsApp/channel contribution branches.

PR strategy note:

- Do not open one giant upstream PR from the combined branch.
- For upstream, split/port into small branches against `upstream/master`.
- The combined branch is useful for confidence, VPS smoke testing, and local evidence only.

## Latest Local/VPS Patch: WebP Sticker Vision Normalization

Problem observed on `tencent-rifuki`:

- After routing media to the configured vision provider, sticker replies reached the bot but vision calls still failed for WhatsApp stickers.
- Latest journal evidence showed `gh/gpt-4o-mini` rejecting the payload with `image media type not supported`.
- The payload was a WhatsApp sticker saved as `image/webp`.

Patch prepared on `codex/v075-combined-channel-fixes`:

- Enable the `image` crate WebP decoder for `zeroclaw-channels` only where image normalization is needed.
- Add an `image-normalization` feature and include it from `whatsapp-web` and `channel-telegram`.
- Convert WebP image attachments to PNG before emitting `[IMAGE:data:...]` markers when vision routing is active.
- Preserve fallback behavior: if WebP decode fails, log `error_key = "media_pipeline_webp_to_png_failed"` and send the original payload instead of dropping the message.

Local validation:

- `cargo test -p zeroclaw-channels --features whatsapp-web media_pipeline --lib` - passed, including `webp_image_is_normalized_to_png_for_vision`.
- `cargo fmt --all -- --check` - passed.
- `cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings` - passed.

CI/CD and VPS:

- Commit: `529944a53` - `fix(channels): normalize webp images for vision`.
- GitHub Actions run: `26841920388` - passed.
- Artifact SHA target: `529944a53fa9cf6c3da9d0de1f7f6804d4536464`.
- VPS host: `tencent-rifuki`.
- Deployed build SHA: `529944a53fa9cf6c3da9d0de1f7f6804d4536464`.
- Service status after restart: `active/running`.
- Backup created: `/home/rifuki/.local/bin/zeroclaw.bak.20260603031556`.
- Startup smoke test: WhatsApp Web connected successfully, saved session loaded, bot phone/LID resolved.

## Latest Local/VPS Patch: Inline Vision Payload History Cleanup

Problem observed on `tencent-rifuki` after moving vision model to `cx/gpt-5.5`:

- Vision works; image/sticker turns can be answered by the configured vision model.
- Remaining "slow response" cases were not provider latency. Logs showed pre-LLM work taking tens of seconds, while `llm_call_ms` was only a few seconds.
- Cause: enriched channel content persisted inline `[IMAGE:data:<mime>;base64,...]` markers into session history. Later turns loaded that payload before provider-layer multimodal trimming could help.

Patch prepared on `codex/v075-combined-channel-fixes`:

- Store compact media history by stripping loadable inline `[IMAGE:data:...]` markers before `append_sender_turn`.
- Restore the full media payload only for the current provider call.
- Strip older/legacy inline image payloads before proactive context compression.
- Keep autosave memory and rollback paths aligned with the compact persisted content.

Local validation:

- `cargo test -p zeroclaw-channels --features whatsapp-web --lib image_payload` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib channel_history_content_for` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib strip_historical_image_payloads` - passed.
- `cargo test -p zeroclaw-channels --features whatsapp-web --lib e2e_failed_vision_turn_does_not_poison_follow_up_text_turn` - passed.
- `cargo fmt --all -- --check` - passed.
- `cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings` - passed.

CI/CD and VPS:

- Commit: `9fa09bd46` - `fix(channels): avoid persisting inline vision payloads`.
- GitHub Actions run: `26845180965` - passed.
- Artifact SHA target: `9fa09bd461fd09a88cc26551b02f48ecb41853ae`.
- VPS host: `tencent-rifuki`.
- Deployed build SHA: `9fa09bd461fd09a88cc26551b02f48ecb41853ae`.
- Service status after restart: `active/running`.
- Backup created: `/home/rifuki/.local/bin/zeroclaw.bak.20260603041930`.
- Live config: `[multimodal].vision_model = "cx/gpt-5.5"`.
- Startup smoke test: WhatsApp Web connected successfully, Telegram listening.
- VPS config file permission fixed to `600`.

## Upstream PR Template Reminders

- One concern per PR.
- Base branch: `master`.
- Use conventional commit title.
- Fill:
  - Summary.
  - Scope boundary.
  - Blast radius.
  - Linked issue.
  - Validation evidence with literal command output.
  - Security/privacy impact.
  - Compatibility.
  - Rollback.
- Never include real phone numbers, group IDs, message bodies, secrets, or personal identifiers in issue/PR logs.

## Upstream Contribution Packaging Contract

Use this checklist before opening any issue or PR upstream. Maintainers use a five-minute intake; missing template sections, unclear scope, weak validation, or privacy mistakes can block review before code is read.

### Issue Format

Use the repo issue template, not a blank issue.

Bug issue title:

- `[Bug]: <clear current broken behavior>`

Required bug fields:

- Affected component: usually `channel` or `provider`.
- Severity: usually `S2 - degraded behavior` for the current fixes; use `S1` only if a workflow is actually blocked.
- Current behavior.
- Expected behavior.
- Steps to reproduce.
- Impact.
- Logs / stack traces: use redacted, neutral logs only.
- ZeroClaw version: commit SHA if testing a local build.
- OS.
- Regression: `Unknown` unless we have proof.
- Pre-flight checks: latest master/latest release reproduced; secrets/PII redacted.

Feature issue title:

- `[Feature]: <requested capability>`

Use feature issue, not bug, for group session scope / passive group observation.

### PR Title

Use Conventional Commits and mirror the squash commit:

- `fix(channels): avoid persisting inline vision payloads in history`
- `fix(channels): normalize webp images for vision`
- `fix(providers): respect explicit vision capability for custom providers`
- `fix(channels/whatsapp-web): allow sticker replies to reach reply gating`
- `fix(channels/whatsapp-web): mark accepted inbound messages read`
- `fix(channels): start typing before slow pre-LLM preparation`
- `fix(channels/whatsapp-web): persist quoted media and document attachments`

Do not put labels in the title. Only include `(#issue)` in the title when that pattern is clearly useful, as in accepted PR `fix(providers): omit temperature for kimi-k2 models in compatible.rs (#7022)`.

### PR Body

Fill `.github/pull_request_template.md` completely:

- Summary:
  - Base branch: `master`.
  - What changed and why: 2-5 bullets.
  - Scope boundary: what this PR explicitly does not change.
  - Blast radius: affected subsystem and why risk is low/medium/high.
  - Linked issue(s): `Fixes #...`, `Related #...`, `Depends on #...`, or `Supersedes #...`.
- Validation Evidence:
  - Paste literal command output or concise tail/result lines, not just "passed".
  - Include local commands plus manual verification.
  - If a full workspace command is skipped, explain why.
- Security & Privacy Impact:
  - Answer all Yes/No lines.
  - Explain any Yes in 1-2 sentences.
- Compatibility:
  - Backward compatible?
  - Config/env/CLI surface changed?
  - Exact upgrade steps if needed.
- Rollback:
  - Low risk: `git revert <sha>` can be enough.
  - Medium/high risk: include command/path, toggles if any, and observable failure symptoms.
- Supersede Attribution:
  - Fill only if using `Supersedes #...`.

Labels belong in GitHub UI, not the body. Expected labels are usually auto-applied by changed paths; verify `risk:*`, `size:*`, and scope labels after opening. If auto-labels are wrong, mention it in a comment rather than hiding it in the PR body.

### First PR Body Draft: Inline Vision History

Use this only after porting the patch cleanly onto fresh `upstream/master` and replacing validation snippets with actual output.

Title:

`fix(channels): avoid persisting inline vision payloads in history`

Body:

````markdown
## Summary

- **Base branch:** `master`
- **What changed and why:**
  - Stores a compact channel-history version of media user turns by removing loadable `[IMAGE:data:...]` payloads before persistence.
  - Restores the full image payload only for the current provider request, so the active vision turn still receives the image bytes.
  - Strips legacy inline image payloads from older restored history before proactive context trimming/compression, preventing later turns from paying base64 history cost.
- **Scope boundary:** This does not change media download/persistence, provider image parsing, model routing, or context-compression policy beyond removing raw inline image payloads from channel history.
- **Blast radius:** Medium. This touches channel orchestration history handling for multimodal messages. Non-image text turns and current-turn vision payload delivery should remain unchanged.
- **Linked issue(s):** Fixes #<issue>

## Validation Evidence (required)

```bash
$ cargo fmt --all -- --check
<paste output>

$ cargo test -p zeroclaw-channels --features whatsapp-web --lib image_payload
<paste output>

$ cargo test -p zeroclaw-channels --features whatsapp-web --lib channel_history_content_for
<paste output>

$ cargo test -p zeroclaw-channels --features whatsapp-web --lib strip_historical_image_payloads
<paste output>

$ cargo test -p zeroclaw-channels --features whatsapp-web --lib e2e_failed_vision_turn_does_not_poison_follow_up_text_turn
<paste output>

$ cargo clippy -p zeroclaw-channels --features whatsapp-web --lib --tests -- -D warnings
<paste output>
```

- **Beyond CI — what did you manually verify?** Verified on a WhatsApp Web deployment with a redacted sticker/image scenario: the active turn still reaches a vision-capable model, and later turns no longer spend tens of seconds in pre-LLM context work due to prior inline image base64 in history.
- **If any command was intentionally skipped, why:** Full workspace `cargo test` was not run for the focused port; targeted channel tests and clippy were run for the affected crate. CI will run the repository gate.

## Security & Privacy Impact (required)

- New permissions, capabilities, or file system access scope? No
- New external network calls? No
- Secrets / tokens / credentials handling changed? No
- PII, real identities, or personal data in diff, tests, fixtures, or docs? No
- If any `Yes`, describe the risk and mitigation: N/A

## Compatibility (required)

- Backward compatible? Yes
- Config / env / CLI surface changed? No
- If `No` or `Yes` to either: N/A

## Rollback (required for `risk: medium` and `risk: high`)

- **Fast rollback command/path:** `git revert <merge-sha>`
- **Feature flags or config toggles:** None
- **Observable failure symptoms:** Vision-capable channel turns could again accumulate inline `[IMAGE:data:...]` payloads in history, causing later messages to spend excessive time in context trimming/compression before the LLM call.

## Supersede Attribution (required only when `Supersedes #` is used)

N/A
````
