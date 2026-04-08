# Arvel Lens — Real-Time Mode

**Next step after the video pipeline is proven: apply the same principles to live AI conversation.**

The core insight carries over directly — don't send every frame, only send what changed. The leader algorithm is already single-pass. It was accidentally designed for streaming.

---

## Why This Matters

Current real-time AI vision (GPT-4o voice mode, Gemini Live) sends frames at fixed intervals — typically 1-2fps regardless of whether anything changed. That's:

- **Wasteful**: 90% of frames are the same face in the same room
- **Expensive**: 1 frame/sec × 60 seconds × 1000 tokens/frame = 60K tokens/minute just for vision
- **Slow**: Processing redundant frames adds latency

Arvel Lens approach: send frames **only when something visually new appears**. During a 30-minute conversation, that might be 15-25 frames total instead of 1,800.

---

## Architecture: Streaming Lens

```
Webcam / Screen share (30fps)
  │
  ├─ Sample 1 frame every 1-2 seconds (not every frame)
  │
  ├─ Leader algorithm (same as video pipeline)
  │   │
  │   │  Centroids persist across the entire conversation.
  │   │  Each new frame: check distance to all existing centroids.
  │   │
  │   ├─ Close to existing centroid → SKIP
  │   │   (AI already knows what this looks like)
  │   │
  │   └─ Far from all centroids → SEND
  │       (something visually new appeared)
  │       Create new centroid, send frame to AI
  │
  ├─ Edge density as PRIORITY signal
  │   │
  │   ├─ High edge (≥5%) + new centroid → send IMMEDIATELY
  │   │   (structured content appeared — screen share, document, diagram)
  │   │
  │   └─ Low edge + new centroid → send with slight delay
  │       (environment change — can wait a beat)
  │
  ├─ Centroid decay (NEW for real-time)
  │   │
  │   │  In video: centroids persist forever (video doesn't change)
  │   │  In real-time: old centroids decay over time
  │   │  
  │   │  Why: If you move to a different room 20 minutes later,
  │   │  the AI should get an updated view even though "person at desk"
  │   │  is technically a known centroid. Time-decay ensures gradual
  │   │  changes get captured.
  │   │
  │   └─ Decay rate: centroid relevance halves every 5 minutes
  │       After 15 min, old centroids are effectively forgotten
  │
  └─ Transcript stream (Parakeet, already real-time capable)
      → Continuous speech-to-text alongside vision updates
```

---

## What the AI Receives

A typical 5-minute conversation:

```
[0:00]  FRAME: Person at desk, dark room, microphone, plant
[0:00]  VOICE: "Hey, so I've been thinking about the API design..."
[0:08]  VOICE: "...we need to handle pagination differently..."
[0:22]  VOICE: "...let me pull up the code..."
[0:23]  FRAME: Screen share — VS Code with Python file open        ← leader triggered
[0:24]  VOICE: "...see this endpoint? It returns everything..."
[0:45]  VOICE: "...what if we added cursor-based pagination..."
[0:52]  FRAME: Screen share — same editor, different file visible   ← leader triggered (new file)
[1:10]  VOICE: "...actually let me sketch this out..."
[1:12]  FRAME: Switched to drawing app, blank canvas               ← leader triggered
[1:30]  FRAME: Drawing app with boxes and arrows sketched           ← leader triggered (content appeared)
[2:15]  VOICE: "...so the flow would be client, gateway, service..."
[3:40]  VOICE: "...what do you think about this approach?"
[4:00]  FRAME: Back to person at desk (centroid decayed, refresh)   ← decay triggered
```

**8 frames in 4 minutes.** The AI has full visual context: knows what the person looks like, saw the code, saw the diagram, knows they're back at the desk. Traditional approach would have sent 240 frames (1fps × 240 seconds).

---

## Key Differences from Video Pipeline

| Aspect | Video (batch) | Real-time (streaming) |
|---|---|---|
| Frame source | Pre-extracted scene changes | Live webcam/screen at 0.5-1fps |
| Leader centroids | Persist forever | Decay over time (half-life ~5 min) |
| pHash dedup | Within must-keep set | Across entire conversation session |
| Edge density | Gate (keep/discard) | Priority signal (immediate vs delayed) |
| Chunking | 15-min segments | Not needed (inherently streaming) |
| Output timing | All at once after processing | Frames sent as they're selected |
| Latency budget | Doesn't matter | <100ms from frame capture to send decision |

---

## Centroid Decay: The New Piece

In pre-recorded video, the content is fixed. Frame 1 and frame 1000 exist simultaneously. Centroids should persist because the same diagram might appear at minute 1 and minute 45.

In real-time, the world changes. The same desk at 0:00 and 30:00 might have different items on it. Without decay, the leader algorithm would say "I already know what the desk looks like" and never send an update.

**Decay mechanism:**

```
centroid_relevance = 1.0 × (0.5 ^ (minutes_since_created / half_life))

When relevance drops below 0.1:
  → Remove centroid from the set
  → Next similar frame will be treated as NEW
  → AI gets a fresh view
```

**Half-life options:**
- **2 minutes**: Aggressive refresh. Good for dynamic environments (cooking, lab work, moving around)
- **5 minutes**: Balanced. Good for desk/office conversations
- **15 minutes**: Conservative. Good for lectures/presentations where visuals are stable

Auto-detection: if many new centroids are being created (dynamic scene), shorten half-life. If few new centroids (static scene), lengthen it.

---

## Object Detection: Not Needed (Yet)

The question was whether to add object detection for real-time context. The answer: **the leader algorithm already handles what object detection would provide.**

| Scenario | Object detection says | Leader algorithm does |
|---|---|---|
| You pick up a book | "book detected" | Thumbnail changed → new centroid → send frame |
| Screen share starts | "monitor, code detected" | Massive visual change → new centroid → send frame |
| Someone walks in | "2 persons detected" | Scene changed → new centroid → send frame |
| You move to whiteboard | "whiteboard detected" | Background changed → new centroid → send frame |
| Nothing changes | "person, desk, laptop" (same as before) | Same thumbnail → skip |

Object detection adds ~50-200ms per frame and tells the AI things it can see in the image itself. The leader algorithm adds ~1ms and tells us WHETHER to send the image at all. The cheaper question is the right one.

**Exception**: If Arvel Iris (the description model) exists, object detection labels could enrich the text description: "person now holding red notebook" instead of just sending the frame. But that's an Iris feature, not a Lens feature.

---

## Latency Budget

For real-time to feel responsive, the entire pipeline from frame capture to send/skip decision must complete in <100ms:

```
Frame capture:                    ~1ms
Resize to 64×64 thumbnail:       ~1ms  
Leader distance computation:      ~1ms (compare to all centroids)
Edge density (Canny):             ~3ms
pHash computation:                ~5ms
Send/skip decision:               ~0ms
────────────────────────────────────
Total:                           ~11ms
```

**Easily within budget.** The pipeline is ~11ms per frame at 0.5fps sampling = negligible CPU load.

If Arvel Iris is added for frame description:
```
All above:                       ~11ms
Iris description (INT8, cached): ~55-80ms
────────────────────────────────────
Total:                           ~66-91ms   ← still under 100ms
```

---

## Integration Points

### With voice AI (Claude, GPT-4o, Gemini)
```
User's microphone → Parakeet (speech-to-text, streaming)  ─┐
User's webcam     → Arvel Lens (frame selection, streaming) ├─→ API LLM → response
User's screen     → Arvel Lens (same pipeline)             ─┘
```

### With Arvel Scout (agent orchestration)
- Scout manages the conversation state
- Lens feeds Scout visual context when it changes
- Scout decides whether to act on visual changes or wait for voice

### Dual-stream: webcam + screen share
- Run two independent leader algorithm instances
- Webcam stream: captures the person, their environment, physical objects
- Screen stream: captures code, documents, browser, apps
- Each has its own centroids and decay rates
- Screen share gets shorter decay (content changes faster on screen)

---

## Token Cost Comparison (30-minute conversation)

| Approach | Frames sent | Image tokens | Text tokens (transcript) | Total |
|---|---|---|---|---|
| **1fps naive** | 1,800 | 1,800,000 | ~15,000 | ~1,815,000 |
| **0.5fps naive** | 900 | 900,000 | ~15,000 | ~915,000 |
| **Arvel Lens (images)** | ~25 | ~25,000 | ~15,000 | **~40,000** |
| **Arvel Lens + Iris (text descriptions)** | ~5 images + ~20 text | ~5,000 + ~400 | ~15,000 | **~20,400** |

**Lens alone: 97.8% reduction** vs 1fps naive.
**Lens + Iris: 98.9% reduction.**

---

## Implementation Roadmap

### Phase 1: Prove it works (days)
- [ ] Add streaming mode to `approach_twopass.py` — accept frames from stdin or websocket
- [ ] Add centroid decay with configurable half-life
- [ ] Test with webcam capture (opencv `VideoCapture(0)`)
- [ ] Measure: how many frames sent per minute in normal conversation?

### Phase 2: Dual-stream (week)
- [ ] Separate webcam + screen share leader instances
- [ ] Priority queue: screen changes > environment changes
- [ ] Edge density fast-path for screen share content

### Phase 3: Integration (depends on Iris)
- [ ] If Iris model exists: describe frames as text, only send images for low-confidence
- [ ] Websocket API for external consumers (Scout, voice assistants, custom apps)
- [ ] Configurable: image-only mode, text-only mode, hybrid mode

### Phase 4: Optimization
- [ ] Auto-tune decay rate based on scene dynamism
- [ ] Bandwidth-aware: degrade gracefully on slow connections
- [ ] GPU-free mobile path (already CPU-only, but test on phones)

---

## What This Enables

A conversation where the AI sees your world efficiently:

- **You're coding**: AI sees your editor when you switch files or make significant changes. Doesn't see 1800 frames of you typing.
- **You're cooking**: AI sees each new ingredient/step. Doesn't see 1800 frames of you stirring.
- **You're in a meeting**: AI sees the whiteboard when content appears. Doesn't see 1800 frames of people sitting.
- **You're walking outside**: AI sees when the scene changes (entered a store, crossed a street). Doesn't see 1800 frames of the sidewalk.
- **You're repairing something**: AI sees each step of the repair. Doesn't see you reaching for the same tool 50 times.

The AI gets full visual context with ~2% of the data. The other 98% was redundant.
