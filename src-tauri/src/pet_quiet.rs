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
