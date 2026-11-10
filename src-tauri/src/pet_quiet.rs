// Maintained low-detail native art. Geometry is shared; silhouettes and markings are species-specific.
impl Appearance {
    fn context_pose(self, mut pose: PoseKeyframe, cue: Option<Cue>) -> PoseKeyframe {
        if let Some(cue) = cue {
            match self {
                Self::Dachshund | Self::Samoyed | Self::Collie => {
                    if matches!(
                        cue,
                        Cue::TodoDone | Cue::FocusDone | Cue::FileOpened | Cue::AgentDone
                    ) {
                        pose.tail_angle = (-pose.offset_y * 5).clamp(-30, 30);
                        pose.eye_state = 3;
                    }
                }
                Self::Siamese | Self::BlueCat => {
                    if matches!(cue, Cue::Wake | Cue::ReminderDue) {
                        pose.ear_deflect = 5;
                    }
                    if matches!(cue, Cue::Settle | Cue::FocusPause) {
                        pose.eye_state = 1;
                    }
                }
                Self::RedPanda => {
                    pose.tail_angle = pose.offset_x.saturating_mul(6);
                    if cue == Cue::TodoDone {
                        pose.eye_state = 3;
                    }
                }
                Self::Otter => {
                    pose.rotation_deg /= 2;
                    pose.offset_y /= 2;
                    if matches!(cue, Cue::Settle | Cue::FocusPause) {
                        pose.eye_state = 1;
                    }
                }
                Self::Capybara => {
                    pose.rotation_deg /= 2;
                    pose.offset_y /= 2;
                }
                _ => {}
            }
        }
        pose
    }
}
unsafe fn draw_quiet_pet(c: &Canvas, pet: Appearance, awake: bool, pose: PoseKeyframe) -> bool {
    use Appearance::*;
    use ArtPath::*;
    if matches!(pet, Cat | Calico | Dog | Panda | Husky) {
        return false;
    }
    let ink = rgb(86, 66, 57);
    let cream = rgb(239, 224, 195);
    let coat = match pet {
        Siamese => cream,
        BlueCat => rgb(108, 109, 119),
        Dachshund => rgb(173, 102, 56),
        Samoyed => rgb(238, 231, 220),
        Collie => rgb(59, 53, 57),
        RedPanda => rgb(181, 103, 60),
        Otter => rgb(151, 105, 72),
        _ => rgb(177, 140, 93),
    };
    let dark = match pet {
        Siamese => rgb(94, 73, 64),
        BlueCat => rgb(88, 88, 101),
        _ => ink,
    };
    let wag = i32::from(pose.tail_angle) * 3;
    let ear = i32::from(pose.ear_deflect) * 3;
    let eyes = |left: i32, right: i32, y: i32| {
        let color = if pet == Collie {
            cream
        } else if pet == Siamese {
            rgb(44, 37, 35)
        } else {
            dark
        };
        if (awake || matches!(pet, Siamese | BlueCat) || pose.eye_state == 2)
            && !matches!(pose.eye_state, 1 | 3)
        {
            let iris = if pet == Siamese {
                rgb(133, 166, 182)
            } else if pet == BlueCat {
                rgb(180, 151, 91)
            } else {
                color
            };
            for x in [left, right] {
                c.oval([x - 26, y - 34, x + 26, y + 34], Some(iris), None, 0);
                if matches!(pet, Siamese | BlueCat) {
                    c.oval([x - 8, y - 27, x + 8, y + 27], Some(ink), None, 0);
                }
            }
        } else {
            for x in [left, right] {
                c.path(
                    &[
                        Move(x - 31, y),
                        Curve(x - 12, y + 16, x + 12, y + 16, x + 31, y),
                    ],
                    None,
                    Some(color),
                    18,
                );
            }
        }
    };
    let mouth = |x: i32, y: i32| {
        let dark = if pet == Siamese {
            rgb(44, 37, 35)
        } else {
            dark
        };
        c.path(
            &[Move(x - 28, y), Line(x + 28, y), Line(x, y + 25), Close],
            Some(dark),
            None,
            0,
        );
        c.path(
            &[
                Move(x, y + 25),
                Curve(x - 12, y + 48, x - 32, y + 48, x - 48, y + 34),
                Move(x, y + 25),
                Curve(x + 12, y + 48, x + 32, y + 48, x + 48, y + 34),
            ],
            None,
            Some(dark),
            12,
        );
    };
    if pet == Otter {
        c.path(
            &[
                Move(865, 925),
                Curve(1080, 970, 1200, 970, 1170, 1030),
                Curve(1080, 1060, 915, 1040, 840, 1010),
                Close,
            ],
            Some(coat),
            Some(ink),
            12,
        );
        c.oval([440, 710, 1080, 1030], Some(coat), Some(ink), 12);
        c.oval([650, 900, 980, 1035], Some(cream), None, 0);
        for x in [250, 550] {
            c.oval(
                [x - 55, 650 - ear, x + 55, 770 - ear],
                Some(coat),
                Some(ink),
                12,
            );
            c.oval([x - 30, 675 - ear, x + 30, 737 - ear], Some(cream), None, 0);
        }
        c.oval([215, 690, 680, 1000], Some(coat), Some(ink), 12);
        c.oval([265, 835, 625, 990], Some(cream), None, 0);
        eyes(345, 540, 805);
        mouth(442, 872);
        for x in [330, 550] {
            c.oval([x - 65, 960, x + 65, 1040], Some(coat), Some(ink), 12);
        }
        return true;
    }
    if pet == Capybara {
        c.oval([510, 590, 1080, 1030], Some(coat), Some(ink), 12);
        for x in [380, 680] {
            c.oval(
                [x - 45, 375 - ear, x + 45, 485 - ear],
                Some(coat),
                Some(ink),
                12,
            );
        }
        c.oval([270, 420, 805, 850], Some(coat), Some(ink), 12);
        c.oval([285, 565, 765, 820], Some(cream), None, 0);
        eyes(430, 650, 530);
        mouth(527, 680);
        for x in [520, 880] {
            c.oval([x - 65, 955, x + 65, 1040], Some(dark), None, 0);
        }
        return true;
    }
    let is_cat = matches!(pet, Siamese | BlueCat);
    let dach = pet == Dachshund;
    // Tail motion follows keyframes; it never runs on an idle heartbeat.
    if dach {
        c.path(
            &[
                Move(1010, 890),
                Curve(1100, 875, 1140, 795 + wag, 1120, 735 + wag),
            ],
            None,
            Some(coat),
            55,
        );
    } else {
        c.path(
            &[
                Move(520, 920),
                Curve(250, 1040, 175, 900, 240, 730 + wag),
                Curve(280, 665 + wag, 365, 695 + wag, 335, 765 + wag),
                Curve(300, 865, 390, 900, 530, 820),
                Close,
            ],
            Some(if pet == Siamese { dark } else { coat }),
            Some(ink),
            12,
        );
    }
    if pet == RedPanda {
        for y in [755, 825, 895] {
            c.path(
                &[Move(225, y + wag / 2), Line(340, y + 55 + wag / 2)],
                None,
                Some(cream),
                55,
            );
        }
    }
    if pet == Collie {
        c.oval([225, 715 + wag, 330, 825 + wag], Some(cream), None, 0);
    }
    c.path(
        &[
            Move(470, 980),
            Curve(440, 850, 480, 720, 570, 675),
            Line(800, 650),
            Curve(900, 710, 920, 850, 960, 985),
            Curve(900, 1040, 550, 1060, 470, 980),
            Close,
        ],
        Some(coat),
        Some(ink),
        12,
    );
    if dach {
        c.oval([480, 730, 1090, 1030], Some(coat), Some(ink), 12);
    }
    if pet != BlueCat {
        c.oval([590, 745, 825, 1020], Some(cream), None, 0);
    }
    let paw = if pet == Siamese || pet == RedPanda {
        dark
    } else if pet == BlueCat {
        coat
    } else {
        cream
    };
    for x in [550, 820] {
        c.oval([x - 70, 955, x + 70, 1040], Some(paw), Some(ink), 9);
    }
    if dach {
        c.oval([360, 315, 850, 730], Some(coat), Some(ink), 12);
        for x in [360, 810] {
            c.oval(
                [x - 65, 355 + ear, x + 65, 780 + ear],
                Some(dark),
                Some(ink),
                12,
            );
        }
        c.oval([420, 555, 790, 730], Some(cream), None, 0);
        eyes(495, 705, 520);
        mouth(600, 610);
        return true;
    }
    if pet == RedPanda {
        for x in [465, 920] {
            c.oval(
                [x - 80, 215 - ear, x + 80, 390 - ear],
                Some(cream),
                Some(ink),
                12,
            );
            c.oval([x - 48, 250 - ear, x + 48, 355 - ear], Some(dark), None, 0);
        }
    } else {
        let left_tip = if pet == Collie { 440 } else { 425 };
        c.path(
            &[
                Move(420, 415),
                Line(left_tip, 180 - ear),
                Line(610, 315),
                Line(805, 315),
                Line(970, 190 - ear),
                Line(980, 425),
                Close,
            ],
            Some(if pet == Siamese { dark } else { coat }),
            Some(ink),
            12,
        );
        if pet == Collie {
            c.path(
                &[
                    Move(805, 315),
                    Line(970, 270 + ear),
                    Line(1030, 420 + ear),
                    Line(910, 430),
                    Close,
                ],
                Some(coat),
                Some(ink),
                12,
            );
        } else {
            c.path(
                &[
                    Move(460, 265 - ear / 2),
                    Line(545, 340),
                    Line(465, 370),
                    Close,
                    Move(925, 270 - ear / 2),
                    Line(850, 340),
                    Line(930, 375),
                    Close,
                ],
                Some(rgb(202, 172, 151)),
                None,
                0,
            );
        }
    }
    c.path(
        &[
            Move(410, 450),
            Curve(440, 300, 580, 285, 700, 290),
            Curve(865, 280, 995, 350, 1000, 500),
            Curve(1010, 665, 910, 745, 705, 755),
            Curve(490, 760, 380, 655, 410, 450),
            Close,
        ],
        Some(coat),
        Some(ink),
        12,
    );
    if pet == Siamese {
        c.oval([440, 355, 970, 695], Some(dark), None, 0);
    }
    if pet == Collie {
        c.path(
            &[
                Move(650, 300),
                Line(745, 300),
                Line(800, 620),
                Line(600, 620),
                Close,
            ],
            Some(cream),
            None,
            0,
        );
    }
    c.oval(
        [540, 555, 865, 715],
        Some(if pet == Siamese { dark } else { cream }),
        None,
        0,
    );
    if pet == RedPanda {
        for x in [445, 880] {
            c.oval([x, 510, x + 110, 640], Some(cream), None, 0);
        }
    }
    eyes(570, 825, 480);
    mouth(700, 605);
    if is_cat {
        c.path(
            &[
                Move(545, 610),
                Line(405, 580),
                Move(545, 640),
                Line(405, 650),
                Move(855, 610),
                Line(995, 580),
                Move(855, 640),
                Line(995, 650),
            ],
            None,
            Some(ink),
            10,
        );
    }
    true
}
unsafe fn draw_quiet_token(c: &Canvas, pet: Appearance, due: bool) -> bool {
    use ArtPath::*;
    let token = pet.token();
    if !matches!(token, "apple" | "clam" | "leaf") {
        return false;
    }
    let ink = pet.palette().outline;
    let fill = if due {
        rgb(255, 237, 190)
    } else if token == "leaf" {
        rgb(147, 164, 103)
    } else {
        pet.palette().accent
    };
    match token {
        "apple" => {
            c.path(
                &[
                    Move(155, 140),
                    Curve(75, 70, 45, 155, 80, 225),
                    Curve(105, 275, 140, 240, 155, 240),
                    Curve(190, 265, 235, 245, 245, 185),
                    Curve(265, 115, 205, 90, 155, 140),
                    Close,
                ],
                Some(fill),
                Some(ink),
                12,
            );
            c.path(
                &[
                    Move(155, 142),
                    Line(160, 100),
                    Move(160, 110),
                    Curve(175, 65, 220, 65, 225, 85),
                    Curve(210, 115, 185, 120, 160, 110),
                    Close,
                ],
                Some(rgb(131, 151, 100)),
                Some(ink),
                10,
            );
        }
        "clam" => {
            c.path(
                &[
                    Move(145, 245),
                    Curve(80, 235, 25, 170, 65, 120),
                    Curve(90, 75, 125, 120, 145, 95),
                    Curve(180, 75, 195, 100, 215, 120),
                    Curve(265, 160, 220, 235, 145, 245),
                    Close,
                ],
                Some(fill),
                Some(ink),
                12,
            );
            for x in [85, 145, 205] {
                c.path(&[Move(145, 235), Line(x, 130)], None, Some(ink), 9);
            }
        }
        _ => {
            c.path(
                &[
                    Move(70, 235),
                    Curve(55, 145, 135, 70, 245, 80),
                    Curve(250, 175, 180, 250, 70, 235),
                    Close,
                ],
                Some(fill),
                Some(ink),
                12,
            );
            c.path(
                &[
                    Move(55, 255),
                    Line(205, 115),
                    Move(130, 185),
                    Line(125, 135),
                ],
                None,
                Some(ink),
                10,
            );
        }
    }
    true
}
#[cfg(test)]
mod quiet_pet_tests {
    use super::*;
    #[test]
    fn catalogue_roundtrips_tokens_and_contexts() {
        for pet in crate::pet::APPEARANCES {
            assert_eq!(Appearance::from_key(pet.key()), pet);
            assert!(!pet.label().is_empty());
            assert!(!pet.token().is_empty());
            for cue in Cue::ALL {
                for frame in cue.frames() {
                    let pose = pet.context_pose(*frame, Some(cue));
                    assert!(pose.rotation_deg.abs() <= 30);
                }
            }
        }
        assert_eq!(Appearance::Otter.token(), "clam");
        assert_eq!(Appearance::RedPanda.token(), "apple");
        assert_eq!(Appearance::Capybara.token(), "leaf");
    }
}
