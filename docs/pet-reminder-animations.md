# Reminder animation ideas, one personality per pet

Status: ideas only, not implemented.

## What exists to build them with
- **Keyframes:** `PoseKeyframe` in `pet_art.rs` (move x/y, tilt ±30°, scale, eye state).
- **Cues:** `Cue::ReminderSaved` and `Cue::ReminderDue`.
- **Tokens:** each pet's token overlay (fish, bone, clam, leaf).
- **Limits:** reference-art pets are single images, so they cannot move their ears or tail
  separately. Every idea uses whole-body moves, eyes and token overlays only.

## Three moments
| Moment | When | Budget |
|---|---|---|
| **Saved** | A reminder was just created | One play, under 1 s |
| **Due** | The reminder fires | One play, then a quiet repeat every 20 s until seen |
| **Ignored** | Still unseen 5 minutes after it was due | One bigger play, then back to the Due repeat |

| Pet | Personality | Saved | Due | Ignored |
|---|---|---|---|---|
| Calico | Aloof cat | Slow blink, then tucks a fish token behind itself | Sits up, quick tail-flick wiggle, three paw-taps toward the bubble | Nudges the fish token toward the cursor, one push at a time |
| Dachshund | Digger | Dips down and squishes, "burying" a bone | Stretches tall, bounces twice, holds the bone up | Full-body sausage shimmy side to side |
| Samoyed | Cloud of fluff | Happy squint and a puff (scale 104%) | Soft cloud bounces, squash on each landing | One small spin-sweep of zoomies, then a smile |
| Border collie | Herder | Eyes wide and a quick nod: "got it" | Darts left and right as if herding you toward the bubble, then crouches and stares | Creeps a few pixels toward the cursor on each repeat |
| Otter | Tool user | Hugs a clam to its chest | Floats on its back, tapping the clam: tap-tap-tap with a small bob | Rolls side to side, still holding the clam |
| Capybara | Unbothered | Slow nod, and a leaf lands on its head | Gentle sway, the leaf on its head wobbles | Long yawn (squint plus stretch). The calmest set, good for Quiet mode |
| Shiba Inu | Dramatic | Confident head tilt | Tilt, bounce, tilt the other way | Theatrical flop onto its side until clicked |

## Rules for the implementation
- **Quiet mode:** no motion. Show the static token badge only.
- **Reduced motion:** eye changes and the token only.
- **Timing:** each play is under 1.2 s. Repeats run only while a reminder is due, so the pet
  costs nothing when idle.
- **Assets:** nothing new beyond tiny token overlays. Uses the existing keyframe player.
- **Stopping:** clicking the pet or the bubble marks the reminder seen and stops the repeat.
