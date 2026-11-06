use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;

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
    #[default]
    Cat,
    Dog,
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
            "husky" => Self::Husky,
            _ => Self::Cat,
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

    /// Natural anatomical fur and feature palette (strictly separated from UI chrome).
    pub fn art_palette(self) -> Palette {
        match self {
            // Tuxedo Cat: Near-black coat, crisp off-white bib/paws, charcoal nose patch (no pink).
            Self::Cat => Palette {
                surface: rgb(246, 244, 238),   // soft ivory/off-white cheeks, bib, paws
                ink: rgb(26, 26, 30),          // near-black primary coat & tail
                accent: rgb(52, 50, 56),       // charcoal nose patch & ear depth (no pink fur)
                outline: rgb(16, 16, 20),      // midnight contour
                selection: rgb(224, 192, 102), // composed golden eye glint
            },
            // Shiba Inu: Red-tan fur, clean cream urajiro, black nose, espresso contours.
            Self::Dog => Palette {
                surface: rgb(252, 246, 234),  // urajiro cream cheeks, throat, muzzle
                ink: rgb(32, 22, 16),         // black cocoa nose & eyes
                accent: rgb(212, 122, 44),    // red-tan ginger coat
                outline: rgb(42, 24, 14),     // deep espresso contour
                selection: rgb(244, 172, 78), // warm amber highlight
            },
            // Calico Cat: White breast/paws, asymmetric orange cheek/nose and black cheek.
            Self::Calico => Palette {
                surface: rgb(250, 248, 244),   // clean ivory white breast, bib, paws
                ink: rgb(28, 26, 30),          // asymmetric jet-black cheek patch
                accent: rgb(226, 120, 48),     // asymmetric warm ginger-orange patch & nose
                outline: rgb(22, 18, 20),      // dark sable contour
                selection: rgb(130, 195, 120), // curious emerald eye glint
            },
            // Smithsonian Giant Panda: Ivory body/head, velvety black limbs, ears, and tilted eye patches.
            Self::Panda => Palette {
                surface: rgb(248, 246, 242),   // clean ivory/white head and belly
                ink: rgb(24, 24, 26),          // solid black ears, eye patches, and shoulder band
                accent: rgb(45, 45, 48),       // dark slate nose and paw pads
                outline: rgb(12, 12, 14),      // pitch black contour
                selection: rgb(185, 205, 190), // calm bamboo mist sheen
            },
            // AKC Siberian Husky: Gray-black saddle/cap, clean white mask/paws, subtle icy eye (no blue fur).
            Self::Husky => Palette {
                surface: rgb(248, 250, 254),   // stark arctic white mask, chest, paws
                ink: rgb(38, 42, 52),          // slate-black cap, mantle, and dorsal saddle
                accent: rgb(112, 122, 138),    // mid-tone slate gray shading
                outline: rgb(22, 26, 34),      // midnight slate contour
                selection: rgb(138, 204, 238), // subtle icy iris (fur remains slate/white)
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
            AnimationPhase::Release => Self::neutral_still_pose(),
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

struct Canvas {
    dc: HDC,
    dpi: i32,
    orig_pen: HPEN,
    orig_brush: HBRUSH,
    pen_edge: HPEN,
    pen_thin: HPEN,
    pen_none: HPEN,
}

impl Canvas {
    unsafe fn new(dc: HDC, dpi: i32, outline: COLORREF) -> Self {
        let s2 = ((2 * dpi + 48) / 96).max(1);
        let s1 = ((1 * dpi + 48) / 96).max(1);
        let pen_edge = CreatePen(PS_SOLID, s2, outline);
        let pen_thin = CreatePen(PS_SOLID, s1, outline);
        let pen_none = CreatePen(PS_NULL, 0, COLORREF(0));

        let orig_pen = HPEN(SelectObject(dc, pen_edge.into()).0);
        let orig_brush = HBRUSH(SelectObject(dc, GetStockObject(NULL_BRUSH)).0);

        Self {
            dc,
            dpi,
            orig_pen,
            orig_brush,
            pen_edge,
            pen_thin,
            pen_none,
        }
    }

    #[inline]
    fn s(&self, v: i32) -> i32 {
        (v * self.dpi + 48) / 96
    }

    unsafe fn fill_oval(&self, l: i32, t: i32, r: i32, b: i32, fill: COLORREF, edge: bool) {
        let br = CreateSolidBrush(fill);
        let old_brush = SelectObject(self.dc, br.into());
        SelectObject(
            self.dc,
            if edge { self.pen_edge } else { self.pen_none }.into(),
        );
        let _ = Ellipse(self.dc, self.s(l), self.s(t), self.s(r), self.s(b));
        SelectObject(self.dc, old_brush);
        let _ = DeleteObject(br.into());
    }

    unsafe fn fill_poly(&self, pts: &[(i32, i32)], fill: COLORREF, edge: bool) {
        let gdi_pts: Vec<POINT> = pts
            .iter()
            .map(|&(x, y)| POINT {
                x: self.s(x),
                y: self.s(y),
            })
            .collect();
        let br = CreateSolidBrush(fill);
        let old_brush = SelectObject(self.dc, br.into());
        SelectObject(
            self.dc,
            if edge { self.pen_edge } else { self.pen_none }.into(),
        );
        let _ = Polygon(self.dc, &gdi_pts);
        SelectObject(self.dc, old_brush);
        let _ = DeleteObject(br.into());
    }

    unsafe fn curve(&self, points: &[(i32, i32)], color: COLORREF, outlined: bool) {
        let points: Vec<_> = points.iter().map(|&(x, y)| POINT { x: self.s(x), y: self.s(y) }).collect();
        let brush = CreateSolidBrush(color);
        let previous_brush = SelectObject(self.dc, brush.into());
        let previous_pen = SelectObject(self.dc, if outlined { self.pen_edge } else { self.pen_none }.into());
        let _ = BeginPath(self.dc);
        let _ = MoveToEx(self.dc, points[0].x, points[0].y, None);
        let _ = PolyBezierTo(self.dc, &points[1..]);
        let _ = CloseFigure(self.dc);
        let _ = EndPath(self.dc);
        let _ = StrokeAndFillPath(self.dc);
        SelectObject(self.dc, previous_pen);
        SelectObject(self.dc, previous_brush);
        let _ = DeleteObject(brush.into());
    }

    unsafe fn line(&self, x1: i32, y1: i32, x2: i32, y2: i32) {
        SelectObject(self.dc, self.pen_thin.into());
        let pts = [
            POINT {
                x: self.s(x1),
                y: self.s(y1),
            },
            POINT {
                x: self.s(x2),
                y: self.s(y2),
            },
        ];
        let _ = Polyline(self.dc, &pts);
    }

    unsafe fn finish(self) {
        SelectObject(self.dc, self.orig_pen.into());
        SelectObject(self.dc, self.orig_brush.into());
        let _ = DeleteObject(self.pen_edge.into());
        let _ = DeleteObject(self.pen_thin.into());
        let _ = DeleteObject(self.pen_none.into());
    }
}

// ============================================================================
// Public Drawing Pipeline
// ============================================================================

/// Primary entry point for rendering the pet in GDI with native DPI scaling.
pub unsafe fn draw_pose(
    dc: HDC,
    appearance: Appearance,
    hover: bool,
    dpi: i32,
    pose: PoseKeyframe,
    file_visible: bool,
) {
    let saved = SaveDC(dc);
    let pixel = dpi as f32 / 96.0;
    let angle = f32::from(pose.rotation_deg).to_radians();
    let scale = f32::from(pose.scale_pct) / 100.0;
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
            eDy: pixel * (64.0 - 64.0 * sine - 64.0 * cosine + f32::from(pose.offset_y)),
        },
    );
    let hover = hover || matches!(pose.eye_state, 1 | 3);
    let pal = appearance.art_palette();
    let c = Canvas::new(dc, dpi, pal.outline);

    match appearance {
        // --------------------------------------------------------------------
        // TUXEDO CAT: Composed & precise. Near-black fur, off-white bib/paws,
        // charcoal nose patch, dark upright tail. No pink coat.
        // --------------------------------------------------------------------
        Appearance::Cat => {
            // Elegant upright curved tail
            c.fill_oval(78, 60, 108, 92, pal.ink, true);
            c.fill_oval(92, 58, 114, 76, pal.ink, true);

            // Torso silhouette
            c.fill_oval(40, 52, 88, 94, pal.ink, true);

            // Off-white bib chest & front paws
            c.fill_oval(54, 58, 74, 88, pal.surface, false);
            c.fill_oval(46, 84, 60, 94, pal.surface, true);
            c.fill_oval(68, 84, 82, 94, pal.surface, true);

            // Pointed cat ears (ink base, charcoal accent interior - no pink)
            c.fill_poly(&[(40, 36), (32, 12), (54, 26)], pal.ink, true);
            c.fill_poly(&[(88, 36), (96, 12), (74, 26)], pal.ink, true);
            c.fill_poly(&[(38, 32), (35, 17), (50, 26)], pal.accent, false);
            c.fill_poly(&[(90, 32), (93, 17), (78, 26)], pal.accent, false);

            // Head silhouette
            c.fill_oval(34, 22, 94, 66, pal.ink, true);

            // Off-white cheeks and clean chevron blaze
            c.fill_oval(38, 44, 56, 62, pal.surface, false);
            c.fill_oval(72, 44, 90, 62, pal.surface, false);
            c.fill_poly(&[(64, 40), (56, 64), (72, 64)], pal.surface, false);

            // Symmetrical crisp whiskers
            c.line(26, 48, 44, 49);
            c.line(26, 53, 44, 51);
            c.line(84, 49, 102, 48);
            c.line(84, 51, 102, 53);

            // Charcoal nose patch (natural dark marking)
            c.fill_poly(&[(62, 50), (66, 50), (64, 53)], pal.accent, false);
            c.line(64, 53, 64, 56);

            // Golden-amber composed eyes with glint or polite hover wink
            if hover {
                c.line(47, 43, 55, 41);
                c.line(55, 41, 47, 39); // wink
                c.fill_oval(72, 38, 82, 48, pal.selection, true);
                c.fill_oval(75, 40, 78, 43, pal.surface, false);
            } else {
                c.fill_oval(46, 38, 56, 48, pal.selection, true);
                c.fill_oval(72, 38, 82, 48, pal.selection, true);
                c.fill_oval(48, 40, 51, 43, pal.surface, false);
                c.fill_oval(74, 40, 77, 43, pal.surface, false);
            }
        }

        // --------------------------------------------------------------------
        // SHIBA INU: Confident & eager. Red-tan coat, cream urajiro cheeks/chest,
        // black nose, alert triangular ears, tightly curled tail.
        // --------------------------------------------------------------------
        Appearance::Dog => {
            // Plump curled tail arched up over the hip with cream underside
            c.fill_oval(78, 46, 106, 74, pal.accent, true);
            c.fill_oval(84, 52, 98, 66, pal.surface, false);

            // Torso & clean cream urajiro chest
            c.fill_oval(40, 52, 88, 94, pal.accent, true);
            c.fill_oval(52, 56, 76, 88, pal.surface, false);

            // Cream front paws
            c.fill_oval(48, 84, 60, 94, pal.surface, true);
            c.fill_oval(68, 84, 80, 94, pal.surface, true);

            // Alert pointed ears with soft cream interior
            c.fill_poly(&[(38, 36), (32, 10), (54, 24)], pal.accent, true);
            c.fill_poly(&[(90, 36), (96, 10), (74, 24)], pal.accent, true);
            c.fill_poly(&[(40, 32), (36, 16), (50, 24)], pal.surface, false);
            c.fill_poly(&[(88, 32), (92, 16), (78, 24)], pal.surface, false);

            // Round chubby cheek head
            c.fill_oval(34, 22, 94, 66, pal.accent, true);

            // Distinctive cream cheek & muzzle urajiro patches
            c.fill_oval(36, 40, 58, 64, pal.surface, false);
            c.fill_oval(70, 40, 92, 64, pal.surface, false);
            c.fill_oval(50, 44, 78, 65, pal.surface, false);

            // Signature Shiba eyebrow dots
            c.fill_oval(48, 30, 54, 35, pal.surface, false);
            c.fill_oval(74, 30, 80, 35, pal.surface, false);

            // Dark cocoa button nose & smiling seam
            c.fill_oval(60, 48, 68, 54, pal.ink, false);
            c.line(64, 54, 64, 57);

            // Confident dark eyes / happy squint with playful tongue on hover
            if hover {
                c.line(46, 42, 51, 38);
                c.line(51, 38, 56, 42); // happy squint
                c.line(72, 42, 77, 38);
                c.line(77, 38, 82, 42);
                c.fill_oval(60, 56, 68, 63, rgb(235, 120, 115), false); // small tongue
            } else {
                c.fill_oval(47, 39, 55, 47, pal.ink, false);
                c.fill_oval(73, 39, 81, 47, pal.ink, false);
                c.fill_oval(49, 41, 52, 43, pal.surface, false);
                c.fill_oval(75, 41, 78, 43, pal.surface, false);
            }
        }

        // --------------------------------------------------------------------
        // CALICO CAT: Curious & playful. White breast and paws, asymmetric orange
        // nose/cheek, black opposite cheek patch, ginger-tipped tail.
        // --------------------------------------------------------------------
        Appearance::Calico => {
            let ivory = rgb(255, 246, 226);
            let shadow = rgb(225, 206, 180);
            let ginger = rgb(198, 112, 48);
            let charcoal = rgb(43, 38, 37);
            let olive = rgb(171, 185, 111);

            // Curled tail wraps around the outside of the seated haunch.
            c.curve(&[(51, 84), (37, 90), (17, 89), (17, 72), (17, 59), (31, 58), (33, 66), (34, 71), (26, 73), (28, 77), (30, 81), (44, 78), (51, 75)], ginger, true);
            c.curve(&[(17, 72), (17, 59), (31, 58), (33, 66), (34, 71), (26, 73), (28, 77), (24, 78), (20, 76), (17, 72)], charcoal, false);

            // The turned haunch, upright chest, and close-set forepaws form one silhouette.
            c.curve(&[(39, 91), (35, 79), (47, 67), (58, 67), (65, 62), (66, 52), (76, 50), (95, 50), (94, 76), (96, 87), (99, 94), (95, 98), (87, 97), (72, 98), (48, 99), (39, 91)], ivory, true);
            c.curve(&[(41, 89), (38, 81), (47, 72), (58, 72), (66, 73), (68, 78), (62, 81), (55, 88), (52, 93), (45, 92)], ginger, false);
            c.curve(&[(43, 88), (46, 85), (49, 84), (55, 85), (57, 89), (53, 92), (47, 93), (45, 92), (44, 90), (43, 88)], charcoal, false);
            c.fill_oval(69, 87, 84, 97, shadow, false);
            c.fill_oval(72, 87, 85, 96, ivory, true);
            c.line(89, 76, 89, 91);

            // An offset head and uneven ears keep the pose curious in silhouette.
            c.curve(&[(48, 39), (45, 30), (43, 16), (46, 13), (51, 12), (59, 21), (64, 24), (72, 21), (82, 22), (87, 23), (94, 18), (100, 12), (103, 15), (106, 20), (103, 35), (101, 41), (112, 56), (99, 67), (82, 70), (64, 72), (44, 64), (44, 52), (44, 47), (46, 42), (48, 39)], ivory, true);
            c.curve(&[(49, 39), (47, 30), (45, 19), (46, 15), (51, 15), (59, 23), (64, 26), (71, 28), (73, 34), (69, 42), (65, 51), (53, 54), (46, 51), (46, 47), (47, 43), (49, 39)], charcoal, false);
            c.curve(&[(80, 27), (85, 25), (91, 24), (100, 16), (104, 16), (101, 34), (99, 41), (104, 46), (106, 52), (102, 55), (94, 57), (87, 50), (85, 44), (82, 38), (76, 33), (80, 27)], ginger, false);
            c.fill_poly(&[(49, 20), (52, 33), (59, 28)], rgb(118, 83, 72), false);
            c.fill_poly(&[(100, 21), (92, 28), (99, 33)], rgb(232, 159, 108), false);

            c.fill_oval(57, 41, 68, 51, olive, true);
            c.fill_oval(61, 42, 65, 50, charcoal, false);
            c.fill_oval(59, 42, 62, 45, ivory, false);
            if hover {
                c.line(88, 46, 93, 48);
                c.line(93, 48, 98, 44);
            } else {
                c.fill_oval(87, 40, 98, 50, olive, true);
                c.fill_oval(91, 41, 95, 49, charcoal, false);
                c.fill_oval(89, 41, 92, 44, ivory, false);
            }
            c.fill_oval(69, 52, 88, 63, shadow, false);
            c.fill_oval(70, 51, 88, 61, ivory, false);
            c.fill_poly(&[(76, 53), (82, 53), (79, 57)], rgb(104, 61, 54), false);
            c.line(79, 57, 79, 59);
            c.line(79, 59, 83, 60);
            c.line(53, 55, 40, 52);
            c.line(54, 59, 41, 61);
            c.line(98, 56, 112, 53);
            c.line(98, 60, 110, 62);
        }

        // --------------------------------------------------------------------
        // GIANT PANDA: Relaxed & deliberate. Broad white head & body, black
        // rounded cup ears, tilted eye patches, black shoulder vest and limbs.
        // --------------------------------------------------------------------
        Appearance::Panda => {
            // Broad black shoulder vest and hindquarter depth
            c.fill_oval(36, 54, 92, 94, pal.ink, true);

            // Clean ivory/white round belly
            c.fill_oval(46, 58, 82, 92, pal.surface, false);

            // Solid black chunky front limbs & rounded paws
            c.fill_oval(38, 72, 58, 95, pal.ink, true);
            c.fill_oval(70, 72, 90, 95, pal.ink, true);
            c.fill_oval(42, 85, 54, 94, pal.accent, false); // paw pad
            c.fill_oval(74, 85, 86, 94, pal.accent, false);

            // Broad, distinctively rounded black cup ears set wide on crown
            c.fill_oval(26, 12, 48, 34, pal.ink, true);
            c.fill_oval(80, 12, 102, 34, pal.ink, true);

            // Compact broad white head silhouette
            c.fill_oval(32, 20, 96, 68, pal.surface, true);

            // Distinctive tilted teardrop black eye patches angled inward
            c.fill_poly(&[(42, 35), (57, 39), (54, 52), (39, 48)], pal.ink, false);
            c.fill_poly(&[(86, 35), (71, 39), (74, 52), (89, 48)], pal.ink, false);

            // Broad dark nose & calm, gentle mouth seam
            c.fill_oval(59, 50, 69, 56, pal.ink, false);
            c.line(64, 56, 64, 59);
            c.line(60, 59, 68, 59);

            // Calm dark eyes with shiny ivory catchlights or peaceful hover squint
            if hover {
                c.line(44, 44, 51, 41);
                c.line(51, 41, 54, 45); // relaxed squint
                c.line(84, 44, 77, 41);
                c.line(77, 41, 74, 45);
            } else {
                c.fill_oval(44, 41, 51, 48, pal.outline, false);
                c.fill_oval(77, 41, 84, 48, pal.outline, false);
                c.fill_oval(46, 42, 48, 44, pal.surface, false);
                c.fill_oval(79, 42, 81, 44, pal.surface, false);
            }
        }

        // --------------------------------------------------------------------
        // SIBERIAN HUSKY: Expressive & energetic. Slate-black cap and mantle,
        // clean white mask/chest/paws, alert pointed ears, tapered brush tail,
        // subtle icy eyes (no blue fur).
        // --------------------------------------------------------------------
        Appearance::Husky => {
            // Tapered brush tail (slate dorsal coat with white underside plume)
            c.fill_oval(78, 54, 110, 84, pal.ink, true);
            c.fill_oval(84, 60, 102, 78, pal.surface, false);

            // Slate-black body saddle with clean arctic white chest
            c.fill_oval(38, 52, 90, 94, pal.ink, true);
            c.fill_oval(50, 56, 78, 90, pal.surface, false);

            // Crisp white athletic paws
            c.fill_oval(46, 84, 60, 94, pal.surface, true);
            c.fill_oval(68, 84, 82, 94, pal.surface, true);

            // Tall, sharply pointed ears: dark slate outer shell, ivory inner lining
            c.fill_poly(&[(38, 34), (30, 8), (54, 22)], pal.ink, true);
            c.fill_poly(&[(90, 34), (98, 8), (74, 22)], pal.ink, true);
            c.fill_poly(&[(40, 30), (34, 14), (50, 22)], pal.surface, false);
            c.fill_poly(&[(88, 30), (94, 14), (78, 22)], pal.surface, false);

            // Slate-black head cap silhouette
            c.fill_oval(34, 22, 94, 66, pal.ink, true);

            // Crisp white Siberian mask (cheek flares & muzzle blaze)
            c.fill_oval(36, 40, 60, 64, pal.surface, false);
            c.fill_oval(68, 40, 92, 64, pal.surface, false);
            c.fill_poly(&[(64, 30), (57, 52), (71, 52)], pal.surface, false);
            c.fill_oval(52, 46, 76, 64, pal.surface, false);

            // Jet-black nose and energetic muzzle seam
            c.fill_oval(60, 48, 68, 54, pal.outline, false);
            c.line(64, 54, 64, 57);

            // Subtle icy blue eyes (iris only; fur remains slate/ivory) & hover pant
            if hover {
                c.fill_oval(46, 38, 56, 48, pal.selection, true);
                c.fill_oval(72, 38, 82, 48, pal.selection, true);
                c.fill_oval(48, 40, 51, 43, pal.surface, false);
                c.fill_oval(74, 40, 77, 43, pal.surface, false);
                // Open energetic panting smile
                c.fill_oval(60, 56, 68, 63, rgb(238, 128, 120), false);
            } else {
                c.fill_oval(46, 38, 56, 48, pal.selection, true);
                c.fill_oval(72, 38, 82, 48, pal.selection, true);
                c.fill_oval(48, 40, 51, 43, pal.surface, false);
                c.fill_oval(74, 40, 77, 43, pal.surface, false);
            }
        }
    }

    if file_visible {
        let palette = appearance.palette();
        c.fill_poly(
            &[(54, 73), (70, 73), (76, 79), (76, 95), (54, 95)],
            palette.ink,
            true,
        );
        c.fill_poly(&[(70, 73), (76, 79), (70, 79)], palette.accent, false);
        c.line(59, 84, 71, 84);
        c.line(59, 89, 67, 89);
    }
    c.finish();
    let _ = RestoreDC(dc, saved);
}
