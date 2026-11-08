use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;

const PET_RENDER_SCALE: f32 = 0.9;

/// High-level 5-color palette used for both UI elements and rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub surface: COLORREF,
    pub ink: COLORREF,
    pub accent: COLORREF,
    pub outline: COLORREF,
    pub selection: COLORREF,
}

/// Supported pet appearances including both original and new species.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Appearance {
    Cat,
    Dog,
    #[default]
    Calico,
    Panda,
    Husky,
}

pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}

impl Appearance {
    pub fn key(self) -> &'static str {
        match self {
            Self::Cat => "cat",
            Self::Dog => "dog",
            Self::Calico => "calico",
            Self::Panda => "panda",
            Self::Husky => "husky",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "dog" => Self::Dog,
            "calico" => Self::Calico,
            "panda" => Self::Panda,
            _ => Self::Calico,
        }
    }

    /// Application chrome and UI theme palette.
    pub fn palette(self) -> Palette {
        match self {
            Self::Cat => Palette {
                surface: rgb(43, 30, 49),
                ink: rgb(249, 240, 253),
                accent: rgb(227, 189, 248),
                outline: rgb(109, 77, 127),
                selection: rgb(76, 53, 90),
            },
            Self::Dog => Palette {
                surface: rgb(51, 33, 24),
                ink: rgb(255, 242, 227),
                accent: rgb(255, 196, 125),
                outline: rgb(134, 90, 56),
                selection: rgb(88, 57, 38),
            },
            Self::Calico => Palette {
                surface: rgb(48, 36, 32),
                ink: rgb(253, 247, 242),
                accent: rgb(245, 170, 110),
                outline: rgb(125, 82, 60),
                selection: rgb(86, 58, 44),
            },
            Self::Panda => Palette {
                surface: rgb(32, 36, 38),
                ink: rgb(244, 248, 246),
                accent: rgb(160, 212, 185),
                outline: rgb(75, 92, 85),
                selection: rgb(52, 68, 62),
            },
            Self::Husky => Palette {
                surface: rgb(30, 36, 48),
                ink: rgb(240, 246, 255),
                accent: rgb(148, 210, 248),
                outline: rgb(74, 94, 128),
                selection: rgb(50, 66, 95),
            },
        }
    }
}

// ============================================================================
// Animation Data API
// ============================================================================

/// Concrete renderable transform and feature deformation for one animation step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoseKeyframe {
    pub duration_ms: u16,
    pub offset_x: i8,     // horizontal translation in DPI-relative virtual units
    pub offset_y: i8,     // vertical translation in DPI-relative virtual units
    pub rotation_deg: i8, // pet body tilt (-30 to +30)
    pub scale_pct: u8,    // 100 = 1.0x, 90 = squished, 110 = stretched
    pub ear_deflect: i8,  // ear perk/droop offset (-20 to +20)
    pub tail_angle: i8,   // tail wag angle (-45 to +45)
    pub eye_state: u8,    // 0 = neutral, 1 = wink/blink, 2 = wide/alert, 3 = happy squint
}

/// Animation phases corresponding to the lifecycle of pet drag/interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationPhase {
    Pickup,
    Held,
    Release,
    Activity(Cue),
}

/// A short reaction to an actual user action or committed outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Cue {
    Wake, Settle, FileExpired, FileOpened, FocusStart, FocusPause, FocusResume,
    FocusDone, TodoSaved, TodoDone, ReminderSaved, ReminderDue, AgentStarted,
    AgentDone, AgentAttention,
}

impl Cue {
    pub const ALL: [Self; 15] = [
        Self::Wake, Self::Settle, Self::FileExpired, Self::FileOpened, Self::FocusStart,
        Self::FocusPause, Self::FocusResume, Self::FocusDone, Self::TodoSaved, Self::TodoDone,
        Self::ReminderSaved, Self::ReminderDue, Self::AgentStarted, Self::AgentDone,
        Self::AgentAttention,
    ];

    pub fn frames(self) -> &'static [PoseKeyframe] {
        use Cue::*;
        match self {
            Wake => const { &[beat(65, 0, -2, -3, 101), beat(105, 0, -4, 0, 102), beat(75, 0, -1, 0, 100)] },
            Settle => const { &[beat(85, 0, 0, 0, 100), beat(115, 0, 2, 0, 98)] },
            FileExpired => const { &[beat(105, 0, 1, 4, 99), beat(110, 0, 2, 0, 98)] },
            FileOpened => const { &[beat(75, 3, -1, 6, 100), beat(110, 1, 0, 2, 100)] },
            FocusStart => const { &[beat(80, 0, 2, 0, 98), beat(140, 0, -3, 0, 103)] },
            FocusPause => const { &[beat(105, 0, 1, -4, 99), beat(125, 0, 3, 0, 98)] },
            FocusResume => const { &[beat(75, 0, 1, 2, 98), beat(120, 0, -3, 0, 102)] },
            FocusDone => const { &[beat(60, 0, 2, 0, 97), beat(110, 0, -5, -4, 103), beat(85, 0, 0, 1, 100)] },
            TodoSaved => const { &[beat(80, 0, 2, 5, 100), beat(85, 0, 0, 1, 100)] },
            TodoDone => const { &[beat(70, 0, -3, -4, 101), beat(75, 0, 1, 0, 99), beat(100, 0, -3, 3, 101)] },
            ReminderSaved => const { &[beat(90, 1, -1, 4, 101), beat(105, 0, 1, 0, 100)] },
            ReminderDue => const { &[beat(75, -2, -2, -7, 101), beat(75, 2, -2, 7, 101), beat(110, 0, -1, 0, 101)] },
            AgentStarted => const { &[beat(85, 0, 1, -4, 100), beat(135, 0, -1, -2, 102)] },
            AgentDone => const { &[beat(75, 0, 1, 0, 98), beat(110, 0, -4, 3, 103), beat(100, 0, 0, 0, 100)] },
            AgentAttention => const { &[beat(80, -2, -2, -5, 101), beat(135, 1, -2, 0, 102)] },
        }
    }
}

const fn beat(duration_ms: u16, offset_x: i8, offset_y: i8, rotation_deg: i8, scale_pct: u8) -> PoseKeyframe {
    PoseKeyframe {
        duration_ms, offset_x, offset_y, rotation_deg, scale_pct,
        ear_deflect: 0, tail_angle: 0, eye_state: 0,
    }
}

/// A coherent, named gesture set containing timing and keyframes for all three phases.
#[derive(Clone, Copy, Debug)]
pub struct AnimationSet {
    pub id: u8,
    pub name: &'static str,
    pub trigger_event: &'static str,
    pub description: &'static str,
    pub pickup: &'static [PoseKeyframe],
    pub held: &'static [PoseKeyframe],
    pub release: &'static [PoseKeyframe],
}

impl AnimationSet {
    /// Evaluates the active pose at `elapsed_ms` within the specified phase.
    pub fn evaluate_pose(&self, phase: AnimationPhase, elapsed_ms: u32) -> PoseKeyframe {
        let frames = match phase {
            AnimationPhase::Pickup => self.pickup,
            AnimationPhase::Held => self.held,
            AnimationPhase::Release => self.release,
            AnimationPhase::Activity(cue) => cue.frames(),
        };

        if frames.is_empty() {
            return Self::neutral_still_pose();
        }

        let total_duration: u32 = frames.iter().map(|f| f.duration_ms as u32).sum();
        if total_duration == 0 {
            return frames[0];
        }

        let effective_ms = if phase == AnimationPhase::Held {
            elapsed_ms % total_duration
        } else {
            elapsed_ms.min(total_duration.saturating_sub(1))
        };

        let mut acc = 0u32;
        for frame in frames {
            acc += frame.duration_ms as u32;
            if effective_ms < acc {
                return *frame;
            }
        }
        *frames.last().unwrap_or(&Self::neutral_still_pose())
    }

    /// Still pose used when Reduced Motion is enabled in Windows accessibility.
    pub const fn reduced_motion_pose(phase: AnimationPhase) -> PoseKeyframe {
        match phase {
            AnimationPhase::Pickup => PoseKeyframe {
                duration_ms: 200,
                offset_x: 0,
                offset_y: -6,
                rotation_deg: 0,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: 5,
                eye_state: 2,
            },
            AnimationPhase::Held => PoseKeyframe {
                duration_ms: 1000,
                offset_x: 0,
                offset_y: -8,
                rotation_deg: 0,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
            AnimationPhase::Release | AnimationPhase::Activity(_) => Self::neutral_still_pose(),
        }
    }

    /// Neutral static ground state (used for idle and cancel-return).
    pub const fn neutral_still_pose() -> PoseKeyframe {
        PoseKeyframe {
            duration_ms: 0,
            offset_x: 0,
            offset_y: 0,
            rotation_deg: 0,
            scale_pct: 100,
            ear_deflect: 0,
            tail_angle: 0,
            eye_state: 0,
        }
    }
}

// ----------------------------------------------------------------------------
// Animation Tables (Exactly 5 distinct sets per animal)
// ----------------------------------------------------------------------------

// 1. TUXEDO CAT (Composed, precise, aristocratic)
static CAT_SETS: [AnimationSet; 5] = [
    AnimationSet {
        id: 0,
        name: "Composed Tuck",
        trigger_event: "LeftClickDrag / PrimaryHold",
        description: "Precise vertical hoist, neat curled paws, serene four-point touchdown.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 80,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 98,
                ear_deflect: 5,
                tail_angle: -8,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: -10,
                rotation_deg: 0,
                scale_pct: 104,
                ear_deflect: 2,
                tail_angle: 12,
                eye_state: 0,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 400,
                offset_x: 0,
                offset_y: -10,
                rotation_deg: -1,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 8,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 400,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: 1,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: -8,
                eye_state: 0,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: 0,
                scale_pct: 102,
                ear_deflect: 4,
                tail_angle: 14,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 97,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 1,
        name: "Fastidious Perch",
        trigger_event: "Ctrl+Drag / PrecisionMode",
        description:
            "Deliberate poise with slight tilt, analytical narrow gaze, polished recovery.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 100,
                offset_x: -1,
                offset_y: -4,
                rotation_deg: 2,
                scale_pct: 102,
                ear_deflect: -3,
                tail_angle: 15,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 140,
                offset_x: -2,
                offset_y: -12,
                rotation_deg: 4,
                scale_pct: 101,
                ear_deflect: 4,
                tail_angle: 20,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 350,
                offset_x: -2,
                offset_y: -12,
                rotation_deg: 4,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: 18,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 350,
                offset_x: -2,
                offset_y: -11,
                rotation_deg: 3,
                scale_pct: 100,
                ear_deflect: -1,
                tail_angle: 22,
                eye_state: 1,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 100,
                offset_x: -1,
                offset_y: -4,
                rotation_deg: 1,
                scale_pct: 103,
                ear_deflect: 0,
                tail_angle: 10,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 98,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
    },
    AnimationSet {
        id: 2,
        name: "Aristocratic Siphon",
        trigger_event: "Alt+Drag / HighPriorityTransfer",
        description: "Taut, high suspension with tail hook and instant symmetrical landing.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 70,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: 6,
                tail_angle: -10,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: -14,
                rotation_deg: 0,
                scale_pct: 106,
                ear_deflect: 8,
                tail_angle: -20,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 250,
                offset_x: 0,
                offset_y: -14,
                rotation_deg: 0,
                scale_pct: 102,
                ear_deflect: 6,
                tail_angle: -15,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 250,
                offset_x: 0,
                offset_y: -13,
                rotation_deg: 0,
                scale_pct: 100,
                ear_deflect: 4,
                tail_angle: -25,
                eye_state: 0,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 80,
                offset_x: 0,
                offset_y: -4,
                rotation_deg: 0,
                scale_pct: 104,
                ear_deflect: 2,
                tail_angle: -8,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 96,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 3,
        name: "Zen Crescent",
        trigger_event: "SlowDrag / HoverInspect",
        description: "Unhurried, rhythmic levitation that maintains composed stillness.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 150,
                offset_x: 0,
                offset_y: -5,
                rotation_deg: -2,
                scale_pct: 100,
                ear_deflect: 1,
                tail_angle: 5,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 200,
                offset_x: 0,
                offset_y: -8,
                rotation_deg: -3,
                scale_pct: 101,
                ear_deflect: 0,
                tail_angle: 8,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 500,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: -3,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 10,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 500,
                offset_x: 0,
                offset_y: -8,
                rotation_deg: -2,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 6,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 140,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: -1,
                scale_pct: 101,
                ear_deflect: 0,
                tail_angle: 3,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 160,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
    },
    AnimationSet {
        id: 4,
        name: "Acrobatic Spring",
        trigger_event: "QuickFlick / RapidDrop",
        description: "Snappy lift, swift counter-turn, and elastic cushioned paw contact.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 60,
                offset_x: 1,
                offset_y: -5,
                rotation_deg: -4,
                scale_pct: 96,
                ear_deflect: -6,
                tail_angle: 25,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 2,
                offset_y: -13,
                rotation_deg: -6,
                scale_pct: 108,
                ear_deflect: 8,
                tail_angle: 35,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 200,
                offset_x: 2,
                offset_y: -13,
                rotation_deg: -5,
                scale_pct: 103,
                ear_deflect: 4,
                tail_angle: 30,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 200,
                offset_x: 1,
                offset_y: -12,
                rotation_deg: -3,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: 20,
                eye_state: 0,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 70,
                offset_x: 1,
                offset_y: -2,
                rotation_deg: -1,
                scale_pct: 105,
                ear_deflect: -2,
                tail_angle: 10,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
];

// 2. SHIBA INU (Confident, eager, spirited)
static DOG_SETS: [AnimationSet; 5] = [
    AnimationSet {
        id: 0,
        name: "Eager Air-Trot",
        trigger_event: "LeftClickDrag / PrimaryHold",
        description: "Brisk paws cycling in air, tight curled tail bob, jubilant touchdown bounce.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 80,
                offset_x: 0,
                offset_y: -4,
                rotation_deg: 2,
                scale_pct: 104,
                ear_deflect: 8,
                tail_angle: 15,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: -11,
                rotation_deg: 4,
                scale_pct: 102,
                ear_deflect: 10,
                tail_angle: 25,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 160,
                offset_x: 1,
                offset_y: -11,
                rotation_deg: 3,
                scale_pct: 101,
                ear_deflect: 6,
                tail_angle: 30,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 160,
                offset_x: -1,
                offset_y: -10,
                rotation_deg: -1,
                scale_pct: 99,
                ear_deflect: 8,
                tail_angle: 20,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: 1,
                scale_pct: 105,
                ear_deflect: 4,
                tail_angle: 15,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 1,
        name: "Helicopter Wag",
        trigger_event: "Ctrl+Drag / RapidCarry",
        description:
            "Enthusiastic tail propeller rotation with alert pointed ears and open-mouthed grin.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 70,
                offset_x: 0,
                offset_y: -5,
                rotation_deg: 0,
                scale_pct: 103,
                ear_deflect: 12,
                tail_angle: -20,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: -12,
                rotation_deg: 0,
                scale_pct: 101,
                ear_deflect: 14,
                tail_angle: 30,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 140,
                offset_x: 0,
                offset_y: -12,
                rotation_deg: 1,
                scale_pct: 100,
                ear_deflect: 10,
                tail_angle: -30,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 140,
                offset_x: 0,
                offset_y: -11,
                rotation_deg: -1,
                scale_pct: 100,
                ear_deflect: 10,
                tail_angle: 30,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 80,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 106,
                ear_deflect: 6,
                tail_angle: 10,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 94,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 2,
        name: "Hero Glider",
        trigger_event: "FastDrag / CrossScreenDash",
        description: "Aerodynamic forward lean, ears pinned back eagerly, confident skid landing.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 2,
                offset_y: -6,
                rotation_deg: 6,
                scale_pct: 106,
                ear_deflect: -8,
                tail_angle: 10,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 4,
                offset_y: -14,
                rotation_deg: 10,
                scale_pct: 104,
                ear_deflect: -12,
                tail_angle: 20,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 220,
                offset_x: 4,
                offset_y: -14,
                rotation_deg: 9,
                scale_pct: 102,
                ear_deflect: -10,
                tail_angle: 22,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 220,
                offset_x: 3,
                offset_y: -13,
                rotation_deg: 8,
                scale_pct: 101,
                ear_deflect: -8,
                tail_angle: 16,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 1,
                offset_y: -3,
                rotation_deg: 3,
                scale_pct: 104,
                ear_deflect: 2,
                tail_angle: 10,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 96,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 3,
        name: "Vigilant Scout",
        trigger_event: "SlowDrag / InspectTarget",
        description: "Tall upright posture with perky head swivels and focused amber eyes.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: -4,
                rotation_deg: -2,
                scale_pct: 102,
                ear_deflect: 6,
                tail_angle: 10,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 150,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: 0,
                scale_pct: 103,
                ear_deflect: 10,
                tail_angle: 15,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 350,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: -3,
                scale_pct: 101,
                ear_deflect: 12,
                tail_angle: 15,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 350,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: 3,
                scale_pct: 101,
                ear_deflect: 12,
                tail_angle: 15,
                eye_state: 2,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: 0,
                scale_pct: 102,
                ear_deflect: 4,
                tail_angle: 8,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 98,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
    },
    AnimationSet {
        id: 4,
        name: "Springboard Pounce",
        trigger_event: "QuickFlick / TargetSnap",
        description: "Deep anticipation crouch, explosive upward pop, and energetic tail whip.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 60,
                offset_x: 0,
                offset_y: 2,
                rotation_deg: -2,
                scale_pct: 92,
                ear_deflect: -4,
                tail_angle: -10,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: -15,
                rotation_deg: 4,
                scale_pct: 112,
                ear_deflect: 14,
                tail_angle: 35,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 180,
                offset_x: 0,
                offset_y: -15,
                rotation_deg: 3,
                scale_pct: 104,
                ear_deflect: 10,
                tail_angle: 25,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 180,
                offset_x: 0,
                offset_y: -13,
                rotation_deg: 1,
                scale_pct: 101,
                ear_deflect: 8,
                tail_angle: 15,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 70,
                offset_x: 0,
                offset_y: -4,
                rotation_deg: 0,
                scale_pct: 108,
                ear_deflect: 4,
                tail_angle: 10,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 92,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
];

// 3. CALICO CAT (Curious, playful, whimsical)
static CALICO_SETS: [AnimationSet; 5] = [
    AnimationSet {
        id: 0,
        name: "Curious Swat",
        trigger_event: "LeftClickDrag / PrimaryHold",
        description:
            "Asymmetric head tilt, curious mid-air batting paw, and bouncy four-paw landing.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: -2,
                offset_y: -3,
                rotation_deg: 4,
                scale_pct: 102,
                ear_deflect: 8,
                tail_angle: -12,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 130,
                offset_x: -4,
                offset_y: -11,
                rotation_deg: 7,
                scale_pct: 103,
                ear_deflect: 12,
                tail_angle: 20,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 280,
                offset_x: -4,
                offset_y: -11,
                rotation_deg: 8,
                scale_pct: 100,
                ear_deflect: 10,
                tail_angle: 24,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 280,
                offset_x: -3,
                offset_y: -10,
                rotation_deg: 5,
                scale_pct: 101,
                ear_deflect: 4,
                tail_angle: -14,
                eye_state: 1,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: -1,
                offset_y: -3,
                rotation_deg: 2,
                scale_pct: 104,
                ear_deflect: 2,
                tail_angle: 8,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 110,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 96,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 1,
        name: "Hammock Pendulum",
        trigger_event: "Ctrl+Drag / SlowFloat",
        description: "Relaxed side-to-side body sway with rhythmic tip-of-tail flicks.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 110,
                offset_x: -3,
                offset_y: -5,
                rotation_deg: -5,
                scale_pct: 101,
                ear_deflect: 4,
                tail_angle: -15,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 150,
                offset_x: 3,
                offset_y: -10,
                rotation_deg: 5,
                scale_pct: 100,
                ear_deflect: 4,
                tail_angle: 20,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 380,
                offset_x: -3,
                offset_y: -10,
                rotation_deg: -4,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: -20,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 380,
                offset_x: 3,
                offset_y: -10,
                rotation_deg: 4,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: 20,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 1,
                offset_y: -3,
                rotation_deg: 1,
                scale_pct: 102,
                ear_deflect: 0,
                tail_angle: 5,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 98,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
    },
    AnimationSet {
        id: 2,
        name: "Spiral Wiggle",
        trigger_event: "Alt+Drag / ExpressiveTransfer",
        description: "Playful body twist with independent alternating ear twitches.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 70,
                offset_x: 2,
                offset_y: -4,
                rotation_deg: -4,
                scale_pct: 97,
                ear_deflect: -8,
                tail_angle: 15,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 100,
                offset_x: -2,
                offset_y: -12,
                rotation_deg: 6,
                scale_pct: 105,
                ear_deflect: 12,
                tail_angle: -25,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 200,
                offset_x: 2,
                offset_y: -12,
                rotation_deg: -5,
                scale_pct: 102,
                ear_deflect: -6,
                tail_angle: 25,
                eye_state: 1,
            },
            PoseKeyframe {
                duration_ms: 200,
                offset_x: -2,
                offset_y: -11,
                rotation_deg: 5,
                scale_pct: 102,
                ear_deflect: 10,
                tail_angle: -25,
                eye_state: 2,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 80,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: 1,
                scale_pct: 104,
                ear_deflect: 2,
                tail_angle: 6,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 3,
        name: "Peekaboo Tease",
        trigger_event: "HoverPause / SoftHold",
        description: "Coy forward crouch, wide inquisitive eyes, and a mischievous wink.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 2,
                scale_pct: 96,
                ear_deflect: -4,
                tail_angle: -8,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 160,
                offset_x: 0,
                offset_y: -8,
                rotation_deg: -2,
                scale_pct: 100,
                ear_deflect: 6,
                tail_angle: 12,
                eye_state: 1,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 450,
                offset_x: 1,
                offset_y: -8,
                rotation_deg: -3,
                scale_pct: 99,
                ear_deflect: 4,
                tail_angle: 14,
                eye_state: 1,
            },
            PoseKeyframe {
                duration_ms: 450,
                offset_x: -1,
                offset_y: -8,
                rotation_deg: 2,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: -10,
                eye_state: 2,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 102,
                ear_deflect: 0,
                tail_angle: 4,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 98,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 4,
        name: "Pounce Skitter",
        trigger_event: "QuickFlick / RapidRelease",
        description: "Energetic hop with flared whiskers, eager tail flick, and soft rebound.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 60,
                offset_x: 1,
                offset_y: -5,
                rotation_deg: -3,
                scale_pct: 108,
                ear_deflect: 10,
                tail_angle: -20,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 2,
                offset_y: -14,
                rotation_deg: 5,
                scale_pct: 105,
                ear_deflect: 14,
                tail_angle: 30,
                eye_state: 2,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 170,
                offset_x: 2,
                offset_y: -14,
                rotation_deg: 4,
                scale_pct: 102,
                ear_deflect: 8,
                tail_angle: 25,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 170,
                offset_x: 1,
                offset_y: -13,
                rotation_deg: -2,
                scale_pct: 100,
                ear_deflect: 6,
                tail_angle: -15,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 70,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: 0,
                scale_pct: 106,
                ear_deflect: 2,
                tail_angle: 8,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 94,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
];

// 4. GIANT PANDA (Relaxed, deliberate, heavyweight)
static PANDA_SETS: [AnimationSet; 5] = [
    AnimationSet {
        id: 0,
        name: "Deliberate Hoist",
        trigger_event: "LeftClickDrag / PrimaryHold",
        description:
            "Substantial, weighty upward drift, relaxed paw dangle, and deep cushioned settle.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 130,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 96,
                ear_deflect: -2,
                tail_angle: 0,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 180,
                offset_x: 0,
                offset_y: -7,
                rotation_deg: 1,
                scale_pct: 102,
                ear_deflect: 2,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 500,
                offset_x: 0,
                offset_y: -7,
                rotation_deg: 1,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 500,
                offset_x: 0,
                offset_y: -6,
                rotation_deg: -1,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 130,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 102,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 170,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 94,
                ear_deflect: -3,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 1,
        name: "Bamboo Cling",
        trigger_event: "Ctrl+Drag / SecureTransfer",
        description:
            "Snug hugging posture as if clutching a sturdy stalk, eyes squinted in content.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 140,
                offset_x: 1,
                offset_y: -3,
                rotation_deg: 2,
                scale_pct: 95,
                ear_deflect: -4,
                tail_angle: 0,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 190,
                offset_x: 2,
                offset_y: -9,
                rotation_deg: 4,
                scale_pct: 101,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 450,
                offset_x: 2,
                offset_y: -9,
                rotation_deg: 4,
                scale_pct: 98,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 450,
                offset_x: 2,
                offset_y: -8,
                rotation_deg: 3,
                scale_pct: 98,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 120,
                offset_x: 1,
                offset_y: -2,
                rotation_deg: 1,
                scale_pct: 101,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 160,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 2,
        name: "Zen Pendulum",
        trigger_event: "SlowDrag / LowEffortFloat",
        description: "Gentle wide-angle rocking with heavy momentum and slow ear wobbles.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 150,
                offset_x: -2,
                offset_y: -3,
                rotation_deg: -3,
                scale_pct: 98,
                ear_deflect: -3,
                tail_angle: 0,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 220,
                offset_x: 2,
                offset_y: -8,
                rotation_deg: 3,
                scale_pct: 100,
                ear_deflect: 3,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 600,
                offset_x: -2,
                offset_y: -8,
                rotation_deg: -3,
                scale_pct: 100,
                ear_deflect: -2,
                tail_angle: 0,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 600,
                offset_x: 2,
                offset_y: -8,
                rotation_deg: 3,
                scale_pct: 100,
                ear_deflect: 2,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 140,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 101,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 180,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 96,
                ear_deflect: -2,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 3,
        name: "Belly Float",
        trigger_event: "Alt+Drag / LowSpeedCruise",
        description:
            "Plump round silhouette reclined slightly backward with calm, serene breathing.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 160,
                offset_x: 0,
                offset_y: -3,
                rotation_deg: -2,
                scale_pct: 102,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 210,
                offset_x: 0,
                offset_y: -8,
                rotation_deg: -4,
                scale_pct: 103,
                ear_deflect: 2,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 550,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: -4,
                scale_pct: 102,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 550,
                offset_x: 0,
                offset_y: -7,
                rotation_deg: -3,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 130,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: -1,
                scale_pct: 102,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 170,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: -2,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
    AnimationSet {
        id: 4,
        name: "Cushion Drop",
        trigger_event: "QuickFlick / RapidDrop",
        description:
            "Quick drop into a satisfying, deep squish deceleration without frantic flailing.",
        pickup: &[
            PoseKeyframe {
                duration_ms: 90,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 95,
                ear_deflect: -3,
                tail_angle: 0,
                eye_state: 2,
            },
            PoseKeyframe {
                duration_ms: 130,
                offset_x: 0,
                offset_y: -10,
                rotation_deg: 0,
                scale_pct: 104,
                ear_deflect: 4,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
        held: &[
            PoseKeyframe {
                duration_ms: 300,
                offset_x: 0,
                offset_y: -10,
                rotation_deg: 0,
                scale_pct: 101,
                ear_deflect: 2,
                tail_angle: 0,
                eye_state: 0,
            },
            PoseKeyframe {
                duration_ms: 300,
                offset_x: 0,
                offset_y: -9,
                rotation_deg: 0,
                scale_pct: 100,
                ear_deflect: 0,
                tail_angle: 0,
                eye_state: 0,
            },
        ],
        release: &[
            PoseKeyframe {
                duration_ms: 100,
                offset_x: 0,
                offset_y: -2,
                rotation_deg: 0,
                scale_pct: 105,
                ear_deflect: -2,
                tail_angle: 0,
                eye_state: 3,
            },
            PoseKeyframe {
                duration_ms: 140,
                offset_x: 0,
                offset_y: 0,
                rotation_deg: 0,
                scale_pct: 90,
                ear_deflect: -4,
                tail_angle: 0,
                eye_state: 3,
            },
        ],
    },
];

// 5. SIBERIAN HUSKY (Expressive, energetic, vocal)
static HUSKY_SETS: [AnimationSet; 5] = [
    AnimationSet {
        id: 0,
        name: "Arctic Launch",
        trigger_event: "LeftClickDrag / PrimaryHold",
        description: "Tall springy lift with alert erect ears, tapered tail whip, and athletic shock absorption.",
        pickup: &[
            PoseKeyframe { duration_ms: 70, offset_x: 0, offset_y: -4, rotation_deg: 1, scale_pct: 97, ear_deflect: 10, tail_angle: -15, eye_state: 2 },
            PoseKeyframe { duration_ms: 110, offset_x: 0, offset_y: -13, rotation_deg: 2, scale_pct: 105, ear_deflect: 14, tail_angle: 25, eye_state: 2 },
        ],
        held: &[
            PoseKeyframe { duration_ms: 220, offset_x: 1, offset_y: -13, rotation_deg: 2, scale_pct: 101, ear_deflect: 12, tail_angle: 20, eye_state: 2 },
            PoseKeyframe { duration_ms: 220, offset_x: -1, offset_y: -12, rotation_deg: -1, scale_pct: 100, ear_deflect: 8, tail_angle: -18, eye_state: 3 },
        ],
        release: &[
            PoseKeyframe { duration_ms: 80, offset_x: 0, offset_y: -3, rotation_deg: 0, scale_pct: 104, ear_deflect: 4, tail_angle: 12, eye_state: 3 },
            PoseKeyframe { duration_ms: 100, offset_x: 0, offset_y: 0, rotation_deg: 0, scale_pct: 95, ear_deflect: 0, tail_angle: 0, eye_state: 3 },
        ],
    },
    AnimationSet {
        id: 1,
        name: "Sled Gallop",
        trigger_event: "Ctrl+Drag / HighSpeedTransit",
        description: "Forward pulling stance with extended paws, streaming tail plume, and eager grin.",
        pickup: &[
            PoseKeyframe { duration_ms: 70, offset_x: 2, offset_y: -5, rotation_deg: 5, scale_pct: 105, ear_deflect: -6, tail_angle: 20, eye_state: 2 },
            PoseKeyframe { duration_ms: 100, offset_x: 4, offset_y: -14, rotation_deg: 8, scale_pct: 106, ear_deflect: -10, tail_angle: 35, eye_state: 3 },
        ],
        held: &[
            PoseKeyframe { duration_ms: 160, offset_x: 4, offset_y: -14, rotation_deg: 8, scale_pct: 102, ear_deflect: -8, tail_angle: 30, eye_state: 3 },
            PoseKeyframe { duration_ms: 160, offset_x: 3, offset_y: -13, rotation_deg: 6, scale_pct: 101, ear_deflect: -6, tail_angle: 15, eye_state: 3 },
        ],
        release: &[
            PoseKeyframe { duration_ms: 80, offset_x: 1, offset_y: -3, rotation_deg: 2, scale_pct: 104, ear_deflect: 0, tail_angle: 10, eye_state: 3 },
            PoseKeyframe { duration_ms: 100, offset_x: 0, offset_y: 0, rotation_deg: 0, scale_pct: 96, ear_deflect: 0, tail_angle: 0, eye_state: 3 },
        ],
    },
    AnimationSet {
        id: 2,
        name: "Theatrical Howl",
        trigger_event: "Alt+Drag / LongCarry",
        description: "Dramatic tilted head, chin lifted skyward, subtle howling mouth posture.",
        pickup: &[
            PoseKeyframe { duration_ms: 90, offset_x: 0, offset_y: -4, rotation_deg: -4, scale_pct: 102, ear_deflect: 6, tail_angle: -10, eye_state: 2 },
            PoseKeyframe { duration_ms: 130, offset_x: 0, offset_y: -12, rotation_deg: -8, scale_pct: 106, ear_deflect: 12, tail_angle: -20, eye_state: 3 },
        ],
        held: &[
            PoseKeyframe { duration_ms: 300, offset_x: 0, offset_y: -12, rotation_deg: -9, scale_pct: 104, ear_deflect: 10, tail_angle: -22, eye_state: 3 },
            PoseKeyframe { duration_ms: 300, offset_x: 0, offset_y: -11, rotation_deg: -7, scale_pct: 102, ear_deflect: 8, tail_angle: -16, eye_state: 3 },
        ],
        release: &[
            PoseKeyframe { duration_ms: 90, offset_x: 0, offset_y: -3, rotation_deg: -2, scale_pct: 103, ear_deflect: 2, tail_angle: -6, eye_state: 3 },
            PoseKeyframe { duration_ms: 110, offset_x: 0, offset_y: 0, rotation_deg: 0, scale_pct: 97, ear_deflect: 0, tail_angle: 0, eye_state: 3 },
        ],
    },
    AnimationSet {
        id: 3,
        name: "Blizzard Swivel",
        trigger_event: "SlowDrag / InquisitiveMove",
        description: "Intense arctic blue gaze scanning back and forth with perky, twitching ears.",
        pickup: &[
            PoseKeyframe { duration_ms: 100, offset_x: -2, offset_y: -4, rotation_deg: -3, scale_pct: 101, ear_deflect: 10, tail_angle: 15, eye_state: 2 },
            PoseKeyframe { duration_ms: 140, offset_x: 2, offset_y: -10, rotation_deg: 3, scale_pct: 102, ear_deflect: 12, tail_angle: -15, eye_state: 2 },
        ],
        held: &[
            PoseKeyframe { duration_ms: 320, offset_x: -2, offset_y: -10, rotation_deg: -4, scale_pct: 100, ear_deflect: 10, tail_angle: 18, eye_state: 2 },
            PoseKeyframe { duration_ms: 320, offset_x: 2, offset_y: -10, rotation_deg: 4, scale_pct: 100, ear_deflect: 10, tail_angle: -18, eye_state: 2 },
        ],
        release: &[
            PoseKeyframe { duration_ms: 90, offset_x: 0, offset_y: -3, rotation_deg: 0, scale_pct: 102, ear_deflect: 4, tail_angle: 6, eye_state: 0 },
            PoseKeyframe { duration_ms: 110, offset_x: 0, offset_y: 0, rotation_deg: 0, scale_pct: 98, ear_deflect: 0, tail_angle: 0, eye_state: 0 },
        ],
    },
    AnimationSet {
        id: 4,
        name: "Snowdrift Skid",
        trigger_event: "QuickFlick / RapidDrop",
        description: "High energetic rebound with a wide panting smile and spirited double tail whip.",
        pickup: &[
            PoseKeyframe { duration_ms: 60, offset_x: 1, offset_y: -5, rotation_deg: 3, scale_pct: 95, ear_deflect: -8, tail_angle: -25, eye_state: 2 },
            PoseKeyframe { duration_ms: 90, offset_x: 2, offset_y: -15, rotation_deg: 6, scale_pct: 110, ear_deflect: 14, tail_angle: 35, eye_state: 3 },
        ],
        held: &[
            PoseKeyframe { duration_ms: 160, offset_x: 2, offset_y: -15, rotation_deg: 5, scale_pct: 103, ear_deflect: 10, tail_angle: 30, eye_state: 3 },
            PoseKeyframe { duration_ms: 160, offset_x: 1, offset_y: -13, rotation_deg: 2, scale_pct: 100, ear_deflect: 6, tail_angle: -20, eye_state: 3 },
        ],
        release: &[
            PoseKeyframe { duration_ms: 70, offset_x: 0, offset_y: -3, rotation_deg: 0, scale_pct: 106, ear_deflect: 2, tail_angle: 12, eye_state: 3 },
            PoseKeyframe { duration_ms: 90, offset_x: 0, offset_y: 0, rotation_deg: 0, scale_pct: 93, ear_deflect: 0, tail_angle: 0, eye_state: 3 },
        ],
    },
];

impl Appearance {
    /// Returns the exact slice of five selectable animation sets tailored to this species.
    pub fn animation_sets(self) -> &'static [AnimationSet; 5] {
        match self {
            Self::Cat => &CAT_SETS,
            Self::Dog => &DOG_SETS,
            Self::Calico => &CALICO_SETS,
            Self::Panda => &PANDA_SETS,
            Self::Husky => &HUSKY_SETS,
        }
    }

    /// Selects an animation set by index (0..4).
    pub fn animation_set(self, index: usize) -> Option<&'static AnimationSet> {
        self.animation_sets().get(index)
    }
}

// ============================================================================
// GDI Safe Canvas
// ============================================================================

enum ArtPath {
    Move(i32, i32),
    Line(i32, i32),
    Curve(i32, i32, i32, i32, i32, i32),
    Close,
}

struct Canvas {
    dc: HDC,
    dpi: i32,
}

impl Canvas {
    fn scaled(&self, tenths: i32) -> i32 {
        (tenths * self.dpi + 480).div_euclid(960)
    }

    unsafe fn with_paint(
        &self,
        fill: Option<COLORREF>,
        stroke: Option<COLORREF>,
        width: i32,
        draw: impl FnOnce(),
    ) {
        let brush = fill.map(|color| CreateSolidBrush(color));
        let pen = stroke.map(|color| CreatePen(PS_SOLID, self.scaled(width).max(1), color));
        let old_brush = SelectObject(
            self.dc,
            brush.map_or_else(|| GetStockObject(NULL_BRUSH), Into::into),
        );
        let old_pen = SelectObject(
            self.dc,
            pen.map_or_else(|| GetStockObject(NULL_PEN), Into::into),
        );
        draw();
        SelectObject(self.dc, old_pen);
        SelectObject(self.dc, old_brush);
        if let Some(pen) = pen {
            let _ = DeleteObject(pen.into());
        }
        if let Some(brush) = brush {
            let _ = DeleteObject(brush.into());
        }
    }

    unsafe fn oval(
        &self,
        [l, t, r, b]: [i32; 4],
        fill: Option<COLORREF>,
        stroke: Option<COLORREF>,
        width: i32,
    ) {
        self.with_paint(fill, stroke, width, || {
            let _ = Ellipse(
                self.dc,
                self.scaled(l),
                self.scaled(t),
                self.scaled(r),
                self.scaled(b),
            );
        });
    }

    unsafe fn path(
        &self,
        segments: &[ArtPath],
        fill: Option<COLORREF>,
        stroke: Option<COLORREF>,
        width: i32,
    ) {
        self.with_paint(fill, stroke, width, || {
            let point = |x, y| POINT {
                x: self.scaled(x),
                y: self.scaled(y),
            };
            let _ = BeginPath(self.dc);
            for segment in segments {
                match *segment {
                    ArtPath::Move(x, y) => {
                        let _ = MoveToEx(self.dc, self.scaled(x), self.scaled(y), None);
                    }
                    ArtPath::Line(x, y) => {
                        let _ = LineTo(self.dc, self.scaled(x), self.scaled(y));
                    }
                    ArtPath::Curve(x1, y1, x2, y2, x, y) => {
                        let _ = PolyBezierTo(self.dc, &[point(x1, y1), point(x2, y2), point(x, y)]);
                    }
                    ArtPath::Close => {
                        let _ = CloseFigure(self.dc);
                    }
                }
            }
            let _ = EndPath(self.dc);
            match (fill.is_some(), stroke.is_some()) {
                (true, true) => {
                    let _ = StrokeAndFillPath(self.dc);
                }
                (true, false) => {
                    let _ = FillPath(self.dc);
                }
                (false, true) => {
                    let _ = StrokePath(self.dc);
                }
                (false, false) => {
                    let _ = AbortPath(self.dc);
                }
            }
        });
    }
}

include!("pet_final.rs");

/// Species-specific Todo control, drawn in the pet's logical 96-DPI space.
pub unsafe fn draw_todo_icon(dc: HDC, appearance: Appearance, due: bool) {
    use ArtPath::*;
    let canvas = Canvas { dc, dpi: 96 };
    match appearance {
        Appearance::Dog | Appearance::Husky => {
            let edge = rgb(134, 69, 46);
            let cream = if due {
                rgb(255, 247, 226)
            } else {
                rgb(247, 222, 174)
            };
            canvas.path(
                &[
                    Move(100, 120),
                    Curve(80, 80, 50, 90, 45, 110),
                    Curve(30, 125, 35, 140, 55, 155),
                    Curve(30, 170, 30, 190, 50, 205),
                    Curve(70, 220, 90, 210, 100, 180),
                    Line(210, 180),
                    Curve(220, 210, 245, 220, 265, 200),
                    Curve(285, 180, 280, 165, 260, 155),
                    Curve(280, 140, 280, 120, 265, 105),
                    Curve(245, 85, 220, 90, 210, 120),
                    Close,
                ],
                Some(cream),
                Some(edge),
                12,
            );
            canvas.path(
                &[
                    Move(102, 160),
                    Line(210, 160),
                    Curve(223, 185, 241, 200, 258, 182),
                    Curve(250, 211, 226, 208, 210, 180),
                    Line(100, 180),
                    Curve(82, 212, 58, 217, 49, 193),
                    Curve(69, 201, 83, 183, 102, 160),
                    Close,
                ],
                Some(rgb(219, 184, 134)),
                None,
                0,
            );
            canvas.path(
                &[Move(107, 132), Line(201, 132)],
                None,
                Some(rgb(255, 249, 229)),
                14,
            );
        }
        Appearance::Panda => {
            let edge = rgb(61, 87, 69);
            let stem = if due {
                rgb(224, 232, 178)
            } else {
                rgb(178, 190, 130)
            };
            canvas.path(
                &[
                    Move(85, 75),
                    Curve(95, 65, 125, 65, 135, 75),
                    Line(135, 270),
                    Curve(123, 280, 95, 280, 85, 270),
                    Close,
                    Move(175, 130),
                    Curve(185, 120, 205, 120, 215, 130),
                    Line(215, 275),
                    Curve(205, 285, 185, 285, 175, 275),
                    Close,
                ],
                Some(stem),
                Some(edge),
                10,
            );
            canvas.path(
                &[
                    Move(92, 134),
                    Line(130, 134),
                    Move(92, 207),
                    Line(130, 207),
                    Move(180, 193),
                    Line(211, 193),
                    Move(180, 247),
                    Line(211, 247),
                ],
                None,
                Some(edge),
                12,
            );
            canvas.path(
                &[
                    Move(100, 85),
                    Line(100, 119),
                    Move(100, 150),
                    Line(100, 190),
                    Move(190, 140),
                    Line(190, 179),
                ],
                None,
                Some(rgb(223, 228, 174)),
                10,
            );
            canvas.path(
                &[
                    Move(135, 145),
                    Curve(145, 95, 180, 70, 210, 75),
                    Curve(195, 105, 170, 135, 135, 145),
                    Close,
                    Move(175, 220),
                    Curve(145, 210, 140, 175, 145, 155),
                    Curve(170, 175, 180, 195, 175, 220),
                    Close,
                    Move(215, 170),
                    Curve(220, 130, 245, 115, 265, 120),
                    Curve(255, 150, 240, 165, 215, 170),
                    Close,
                ],
                Some(rgb(113, 137, 87)),
                Some(edge),
                9,
            );
        }
        _ => {
            let edge = rgb(105, 79, 67);
            let body = if due {
                rgb(255, 248, 229)
            } else {
                rgb(255, 240, 214)
            };
            canvas.path(
                &[
                    Move(90, 165),
                    Line(30, 118),
                    Curve(42, 161, 42, 205, 30, 240),
                    Line(90, 205),
                    Close,
                    Move(130, 125),
                    Line(172, 88),
                    Line(203, 126),
                    Close,
                ],
                Some(rgb(199, 110, 64)),
                Some(edge),
                11,
            );
            canvas.oval([65, 115, 280, 255], Some(body), Some(edge), 12);
            canvas.path(
                &[
                    Move(81, 190),
                    Curve(120, 231, 205, 249, 252, 216),
                    Curve(222, 258, 126, 251, 88, 222),
                    Close,
                ],
                Some(rgb(234, 217, 191)),
                None,
                0,
            );
            canvas.path(
                &[
                    Move(104, 128),
                    Curve(133, 117, 159, 120, 170, 133),
                    Curve(169, 154, 148, 160, 130, 151),
                    Curve(115, 145, 108, 139, 104, 128),
                    Close,
                ],
                Some(rgb(199, 110, 64)),
                None,
                0,
            );
            canvas.path(
                &[Move(150, 150), Curve(168, 177, 168, 206, 151, 225)],
                None,
                Some(edge),
                10,
            );
            canvas.oval([225, 153, 244, 172], Some(rgb(56, 48, 55)), None, 0);
        }
    }
}

/// Primary entry point for rendering the pet in GDI with native DPI scaling.
pub unsafe fn draw_pose(
    dc: HDC,
    appearance: Appearance,
    hover: bool,
    dpi: i32,
    pose: PoseKeyframe,
    file_motion: Option<(i32, i32)>,
    awake: bool,
    cue: Option<Cue>,
) {
    let saved = SaveDC(dc);
    let pixel = dpi as f32 / 96.0;
    let angle = f32::from(pose.rotation_deg).to_radians();
    let scale = f32::from(pose.scale_pct) / 100.0 * PET_RENDER_SCALE;
    let (sine, cosine) = angle.sin_cos();
    let (sine, cosine) = (sine * scale, cosine * scale);
    SetGraphicsMode(dc, GM_ADVANCED);
    let _ = SetWorldTransform(
        dc,
        &XFORM {
            eM11: cosine,
            eM12: sine,
            eM21: -sine,
            eM22: cosine,
            eDx: pixel * (64.0 - 64.0 * cosine + 64.0 * sine + f32::from(pose.offset_x)),
            eDy: pixel
                * (64.0 - 64.0 * sine - 64.0 * cosine + f32::from(pose.offset_y)
                    - if hover { 1.0 } else { 0.0 }),
        },
    );

    SetPolyFillMode(dc, WINDING);
    let canvas = Canvas { dc, dpi };
    draw_final_art(&canvas, appearance, awake);
    if let Some((file_x, file_y)) = file_motion {
        use ArtPath::*;
        let palette = appearance.palette();
        let x = file_x + if appearance == Appearance::Panda { 90 } else { 0 };
        let y = file_y;
        let move_to = |px, py| Move(px + x, py + y);
        let line_to = |px, py| Line(px + x, py + y);
        canvas.path(
            &[
                move_to(540, 730),
                line_to(700, 730),
                line_to(760, 790),
                line_to(760, 950),
                line_to(540, 950),
                Close,
            ],
            Some(palette.ink),
            Some(palette.outline),
            20,
        );
        canvas.path(
            &[move_to(700, 730), line_to(760, 790), line_to(700, 790), Close],
            Some(palette.accent),
            None,
            0,
        );
        canvas.path(
            &[
                move_to(590, 840),
                line_to(710, 840),
                move_to(590, 890),
                line_to(670, 890),
            ],
            None,
            Some(palette.outline),
            10,
        );
    }
    let _ = RestoreDC(dc, saved);
    if let Some(cue) = cue {
        draw_cue(&canvas, appearance.palette(), cue);
    }
}


// Keep feedback marks stationary relative to the pet window while the body moves.
unsafe fn draw_cue(canvas: &Canvas, palette: Palette, cue: Cue) {
    use ArtPath::*;
    use Cue::*;
    let mark: &[ArtPath] = match cue {
        FocusStart | FocusResume => &[Move(1070, 100), Line(1070, 223), Line(1180, 162), Close],
        FocusPause => &[Move(1080, 106), Line(1080, 222), Move(1150, 106), Line(1150, 222)],
        FocusDone | TodoDone | AgentDone => &[Move(1050, 170), Line(1100, 220), Line(1190, 104)],
        ReminderSaved | ReminderDue => &[Move(1060, 201), Line(1080, 170), Line(1080, 139),
            Curve(1080, 90, 1160, 90, 1160, 139), Line(1160, 170), Line(1180, 201), Close,
            Move(1104, 229), Line(1136, 229)],
        TodoSaved => &[Move(1070, 162), Line(1170, 162), Move(1120, 112), Line(1120, 212)],
        AgentStarted => &[Move(1050, 126), Line(1095, 168), Line(1050, 210), Move(1120, 210), Line(1180, 210)],
        AgentAttention => &[Move(1120, 100), Line(1120, 177), Move(1120, 210), Line(1120, 227)],
        _ => return,
    };
    canvas.oval([1000, 55, 1240, 282], Some(palette.surface), Some(palette.outline), 14);
    canvas.path(mark, None, Some(palette.accent), 20);
}
