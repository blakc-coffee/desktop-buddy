//! One explicit screen-fill action against a revalidated accessible control.
use super::*;
use windows::Win32::{
    Foundation::*,
    System::Com::*,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};
#[derive(Clone, Serialize)]
pub struct ControlTarget {
    pub id: usize,
    pub kind: String,
    pub label: String,
    pub rect: [i32; 4],
}
struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
unsafe fn automation() -> Result<(Com, IUIAutomation), String> {
    CoInitializeEx(None, COINIT_MULTITHREADED)
        .ok()
        .map_err(err)?;
    let guard = Com;
    let api = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(err)?;
    Ok((guard, api))
}
#[allow(non_upper_case_globals)]
unsafe fn kind(element: &IUIAutomationElement) -> Option<&'static str> {
    if !element.CurrentIsEnabled().ok()?.as_bool()
        || element.CurrentIsOffscreen().ok()?.as_bool()
        || element.CurrentIsPassword().ok()?.as_bool()
    {
        return None;
    }
    match element.CurrentControlType().ok()? {
        UIA_EditControlTypeId => Some("text"),
        UIA_RadioButtonControlTypeId => Some("radio"),
        UIA_CheckBoxControlTypeId => Some("checkbox"),
        _ => None,
    }
}
pub unsafe fn collect(window: HWND) -> Result<Vec<ControlTarget>, String> {
    let (_guard, api) = automation()?;
    let root = api.ElementFromHandle(window).map_err(err)?;
    let elements = root
        .FindAll(TreeScope_Subtree, &api.CreateTrueCondition().map_err(err)?)
        .map_err(err)?;
    let mut output = Vec::new();
    for i in 0..elements.Length().map_err(err)?.min(1500) {
        let element = elements.GetElement(i).map_err(err)?;
        if let Some(kind) = kind(&element) {
            let rect = element.CurrentBoundingRectangle().map_err(err)?;
            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }
            let label = element.CurrentName().map_err(err)?.to_string();
            output.push(ControlTarget {
                id: output.len(),
                kind: kind.into(),
                label: label.chars().take(240).collect(),
                rect: [rect.left, rect.top, rect.right, rect.bottom],
            });
            if output.len() == 60 {
                break;
            }
        }
    }
    Ok(output)
}
/// Returns true for text (caller types); false after selecting an option. Buttons are excluded.
pub unsafe fn activate(window: HWND, pid: u32, target: &ControlTarget) -> Result<bool, String> {
    if GetForegroundWindow() != window {
        return Err("The captured app is no longer in front. Capture again.".into());
    }
    let (_guard, api) = automation()?;
    let [left, top, right, bottom] = target.rect;
    let element = api
        .ElementFromPoint(POINT {
            x: left + (right - left) / 2,
            y: top + (bottom - top) / 2,
        })
        .map_err(err)?;
    let rect = element.CurrentBoundingRectangle().map_err(err)?;
    if element.CurrentProcessId().map_err(err)? as u32 != pid
        || kind(&element) != Some(target.kind.as_str())
        || element
            .CurrentName()
            .map_err(err)?
            .to_string()
            .chars()
            .take(240)
            .collect::<String>()
            != target.label
        || [rect.left, rect.top, rect.right, rect.bottom] != target.rect
    {
        return Err("The answer control changed. Nothing was filled; capture again.".into());
    }
    match target.kind.as_str() {
        "text" => {
            element.SetFocus().map_err(err)?;
            Ok(true)
        }
        "radio" => {
            element
                .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                )
                .map_err(err)?
                .Select()
                .map_err(err)?;
            Ok(false)
        }
        "checkbox" => {
            let toggle = element
                .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                .map_err(err)?;
            if toggle.CurrentToggleState().map_err(err)? == ToggleState_Off {
                toggle.Toggle().map_err(err)?;
            }
            Ok(false)
        }
        _ => Err("No supported answer control.".into()),
    }
}
