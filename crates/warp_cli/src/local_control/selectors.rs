//! CLI argument conversion into shared local-control selectors.
use local_control::protocol::{PaneTarget, SessionTarget, TabTarget, WindowTarget};
use local_control::selection::InstanceSelector;
use local_control::{
    ControlError, ErrorCode, PaneSelector, SessionSelector, TabSelector, TargetSelector,
    WindowSelector,
};

use crate::local_control::TargetArgs;

pub(super) fn instance_selector(args: TargetArgs) -> InstanceSelector {
    if let Some(instance_id) = args.instance {
        return InstanceSelector::Id(local_control::discovery::InstanceId(instance_id));
    }
    if let Some(pid) = args.pid {
        return InstanceSelector::Pid(pid);
    }
    InstanceSelector::Active
}

pub(super) fn target_selector(args: &TargetArgs) -> Result<TargetSelector, ControlError> {
    Ok(TargetSelector {
        window: window_target(args)?,
        tab: tab_target(args)?,
        pane: pane_target(args)?,
        session: session_target(args)?,
    })
}

fn window_target(args: &TargetArgs) -> Result<Option<WindowTarget>, ControlError> {
    if let Some(value) = &args.window {
        return parse_window_target(value).map(Some);
    }
    if let Some(id) = &args.window_id {
        return Ok(Some(WindowTarget::Id {
            id: WindowSelector(id.clone()),
        }));
    }
    if let Some(index) = args.window_index {
        return Ok(Some(WindowTarget::Index { index }));
    }
    Ok(args.window_title.as_ref().map(|title| WindowTarget::Title {
        title: title.clone(),
    }))
}

fn tab_target(args: &TargetArgs) -> Result<Option<TabTarget>, ControlError> {
    if let Some(value) = &args.tab {
        return parse_tab_target(value).map(Some);
    }
    if let Some(id) = &args.tab_id {
        return Ok(Some(TabTarget::Id {
            id: TabSelector(id.clone()),
        }));
    }
    if let Some(index) = args.tab_index {
        return Ok(Some(TabTarget::Index { index }));
    }
    Ok(args.tab_title.as_ref().map(|title| TabTarget::Title {
        title: title.clone(),
    }))
}

fn pane_target(args: &TargetArgs) -> Result<Option<PaneTarget>, ControlError> {
    if let Some(value) = &args.pane {
        return parse_pane_target(value).map(Some);
    }
    if let Some(id) = &args.pane_id {
        return Ok(Some(PaneTarget::Id {
            id: PaneSelector(id.clone()),
        }));
    }
    Ok(args.pane_index.map(|index| PaneTarget::Index { index }))
}

fn session_target(args: &TargetArgs) -> Result<Option<SessionTarget>, ControlError> {
    if let Some(value) = &args.session {
        return parse_session_target(value).map(Some);
    }
    if let Some(id) = &args.session_id {
        return Ok(Some(SessionTarget::Id {
            id: SessionSelector(id.clone()),
        }));
    }
    Ok(args
        .session_index
        .map(|index| SessionTarget::Index { index }))
}

fn parse_window_target(value: &str) -> Result<WindowTarget, ControlError> {
    if value == "active" {
        return Ok(WindowTarget::Active);
    }
    if let Some(id) = value.strip_prefix("id:") {
        return Ok(WindowTarget::Id {
            id: WindowSelector(id.to_owned()),
        });
    }
    if let Some(index) = value.strip_prefix("index:") {
        return Ok(WindowTarget::Index {
            index: parse_index(index, "window")?,
        });
    }
    if let Some(title) = value.strip_prefix("title:") {
        return Ok(WindowTarget::Title {
            title: title.to_owned(),
        });
    }
    Err(invalid_selector("window"))
}

fn parse_tab_target(value: &str) -> Result<TabTarget, ControlError> {
    if value == "active" {
        return Ok(TabTarget::Active);
    }
    if let Some(id) = value.strip_prefix("id:") {
        return Ok(TabTarget::Id {
            id: TabSelector(id.to_owned()),
        });
    }
    if let Some(index) = value.strip_prefix("index:") {
        return Ok(TabTarget::Index {
            index: parse_index(index, "tab")?,
        });
    }
    if let Some(title) = value.strip_prefix("title:") {
        return Ok(TabTarget::Title {
            title: title.to_owned(),
        });
    }
    Err(invalid_selector("tab"))
}

fn parse_pane_target(value: &str) -> Result<PaneTarget, ControlError> {
    if value == "active" {
        return Ok(PaneTarget::Active);
    }
    if let Some(id) = value.strip_prefix("id:") {
        return Ok(PaneTarget::Id {
            id: PaneSelector(id.to_owned()),
        });
    }
    if let Some(index) = value.strip_prefix("index:") {
        return Ok(PaneTarget::Index {
            index: parse_index(index, "pane")?,
        });
    }
    Err(invalid_selector("pane"))
}

fn parse_session_target(value: &str) -> Result<SessionTarget, ControlError> {
    if value == "active" {
        return Ok(SessionTarget::Active);
    }
    if let Some(id) = value.strip_prefix("id:") {
        return Ok(SessionTarget::Id {
            id: SessionSelector(id.to_owned()),
        });
    }
    if let Some(index) = value.strip_prefix("index:") {
        return Ok(SessionTarget::Index {
            index: parse_index(index, "session")?,
        });
    }
    Err(invalid_selector("session"))
}

fn parse_index(value: &str, family: &str) -> Result<u32, ControlError> {
    value.parse::<u32>().map_err(|_| invalid_selector(family))
}

fn invalid_selector(family: &str) -> ControlError {
    ControlError::new(
        ErrorCode::InvalidSelector,
        format!("invalid {family} selector"),
    )
}
