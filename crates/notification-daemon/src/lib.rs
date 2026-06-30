#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

impl NotificationUrgency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationAction {
    pub key: &'static str,
    pub label: &'static str,
}

impl NotificationAction {
    pub const fn new(key: &'static str, label: &'static str) -> Self {
        Self { key, label }
    }

    pub fn valid(&self) -> bool {
        !self.key.trim().is_empty() && !self.label.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRequest {
    pub app_name: &'static str,
    pub replaces_id: Option<u64>,
    pub summary: &'static str,
    pub body: &'static str,
    pub urgency: NotificationUrgency,
    pub expire_timeout_ms: Option<u64>,
    pub actions: Vec<NotificationAction>,
}

impl NotificationRequest {
    pub fn new(
        app_name: &'static str,
        summary: &'static str,
        body: &'static str,
        urgency: NotificationUrgency,
    ) -> Self {
        Self {
            app_name,
            replaces_id: None,
            summary,
            body,
            urgency,
            expire_timeout_ms: Some(5_000),
            actions: Vec::new(),
        }
    }

    pub fn replacing(mut self, id: u64) -> Self {
        self.replaces_id = Some(id);
        self
    }

    pub fn persistent(mut self) -> Self {
        self.expire_timeout_ms = None;
        self
    }

    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn spec_fields_valid(&self) -> bool {
        !self.app_name.trim().is_empty()
            && !self.summary.trim().is_empty()
            && self.actions.iter().all(NotificationAction::valid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveNotification {
    pub id: u64,
    pub request: NotificationRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    Expired,
    Dismissed,
    Replaced,
}

impl CloseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::Dismissed => "dismissed",
            Self::Replaced => "replaced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedNotification {
    pub id: u64,
    pub reason: CloseReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokedAction {
    pub id: u64,
    pub key: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDaemon {
    next_id: u64,
    active: Vec<ActiveNotification>,
    closed: Vec<ClosedNotification>,
    actions_invoked: Vec<InvokedAction>,
    notify_calls: u64,
}

impl Default for NotificationDaemon {
    fn default() -> Self {
        Self {
            next_id: 1,
            active: Vec::new(),
            closed: Vec::new(),
            actions_invoked: Vec::new(),
            notify_calls: 0,
        }
    }
}

impl NotificationDaemon {
    pub fn notify(&mut self, request: NotificationRequest) -> u64 {
        self.notify_calls += 1;

        if let Some(replaces_id) = request.replaces_id {
            if let Some(notification) = self
                .active
                .iter_mut()
                .find(|notification| notification.id == replaces_id)
            {
                notification.request = request;
                self.closed.push(ClosedNotification {
                    id: replaces_id,
                    reason: CloseReason::Replaced,
                });
                return replaces_id;
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        self.active.push(ActiveNotification { id, request });
        id
    }

    pub fn invoke_action(&mut self, id: u64, key: &'static str) -> bool {
        let Some(notification) = self
            .active
            .iter()
            .find(|notification| notification.id == id)
        else {
            return false;
        };

        if !notification
            .request
            .actions
            .iter()
            .any(|action| action.key == key)
        {
            return false;
        }

        self.actions_invoked.push(InvokedAction { id, key });
        true
    }

    pub fn expire(&mut self, id: u64) -> bool {
        self.close(id, CloseReason::Expired)
    }

    pub fn dismiss(&mut self, id: u64) -> bool {
        self.close(id, CloseReason::Dismissed)
    }

    pub fn active(&self) -> &[ActiveNotification] {
        &self.active
    }

    pub fn closed(&self) -> &[ClosedNotification] {
        &self.closed
    }

    pub fn actions_invoked(&self) -> &[InvokedAction] {
        &self.actions_invoked
    }

    pub fn notify_calls(&self) -> u64 {
        self.notify_calls
    }

    fn close(&mut self, id: u64, reason: CloseReason) -> bool {
        let Some(index) = self
            .active
            .iter()
            .position(|notification| notification.id == id)
        else {
            return false;
        };

        self.active.remove(index);
        self.closed.push(ClosedNotification { id, reason });
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationSmokeReport {
    pub notify_calls: u64,
    pub active_after_replace: u64,
    pub replacement_preserved_id: bool,
    pub action_invoked: bool,
    pub closed_replaced: bool,
    pub closed_expired: bool,
    pub closed_dismissed: bool,
    pub critical_persistent: bool,
    pub spec_fields_valid: bool,
    pub active_after_cleanup: u64,
}

impl NotificationSmokeReport {
    pub fn passed(self) -> bool {
        self.notify_calls == 3
            && self.active_after_replace == 1
            && self.replacement_preserved_id
            && self.action_invoked
            && self.closed_replaced
            && self.closed_expired
            && self.closed_dismissed
            && self.critical_persistent
            && self.spec_fields_valid
            && self.active_after_cleanup == 0
    }
}

pub fn run_notification_smoke() -> NotificationSmokeReport {
    let first_request = NotificationRequest::new(
        "Backlit Terminal",
        "Build started",
        "Running cargo test --workspace",
        NotificationUrgency::Normal,
    )
    .with_action(NotificationAction::new("default", "Open"));

    let replacement_template = NotificationRequest::new(
        "Backlit Terminal",
        "Build passed",
        "All workspace tests completed",
        NotificationUrgency::Normal,
    )
    .with_action(NotificationAction::new("default", "Open"));

    let critical_request = NotificationRequest::new(
        "Backlit Power",
        "Battery critical",
        "Connect power to avoid suspend",
        NotificationUrgency::Critical,
    )
    .persistent()
    .with_action(NotificationAction::new("settings", "Power settings"));

    let spec_fields_valid = first_request.spec_fields_valid()
        && replacement_template.spec_fields_valid()
        && critical_request.spec_fields_valid();

    let mut daemon = NotificationDaemon::default();
    let first_id = daemon.notify(first_request);
    let replacement_id = daemon.notify(replacement_template.replacing(first_id));
    let active_after_replace = daemon.active().len() as u64;
    let replacement_preserved_id = replacement_id == first_id;
    let action_invoked = daemon.invoke_action(replacement_id, "default");
    let expired = daemon.expire(replacement_id);

    let critical_id = daemon.notify(critical_request);
    let critical_persistent = daemon.active().iter().any(|notification| {
        notification.id == critical_id
            && notification.request.urgency == NotificationUrgency::Critical
            && notification.request.expire_timeout_ms.is_none()
    });
    let dismissed = daemon.dismiss(critical_id);

    let closed_replaced = daemon
        .closed()
        .iter()
        .any(|closed| closed.id == first_id && closed.reason == CloseReason::Replaced);
    let closed_expired = expired
        && daemon
            .closed()
            .iter()
            .any(|closed| closed.id == replacement_id && closed.reason == CloseReason::Expired);
    let closed_dismissed = dismissed
        && daemon
            .closed()
            .iter()
            .any(|closed| closed.id == critical_id && closed.reason == CloseReason::Dismissed);

    NotificationSmokeReport {
        notify_calls: daemon.notify_calls(),
        active_after_replace,
        replacement_preserved_id,
        action_invoked,
        closed_replaced,
        closed_expired,
        closed_dismissed,
        critical_persistent,
        spec_fields_valid,
        active_after_cleanup: daemon.active().len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        run_notification_smoke, CloseReason, NotificationAction, NotificationDaemon,
        NotificationRequest, NotificationUrgency,
    };

    #[test]
    fn notification_smoke_passes() {
        let report = run_notification_smoke();

        assert!(report.passed(), "{report:?}");
    }

    #[test]
    fn replacement_preserves_notification_id_and_records_close() {
        let mut daemon = NotificationDaemon::default();
        let first = daemon.notify(NotificationRequest::new(
            "App",
            "First",
            "",
            NotificationUrgency::Normal,
        ));
        let replacement = daemon.notify(
            NotificationRequest::new("App", "Second", "", NotificationUrgency::Normal)
                .replacing(first),
        );

        assert_eq!(first, replacement);
        assert_eq!(daemon.active().len(), 1);
        assert_eq!(daemon.closed()[0].reason, CloseReason::Replaced);
    }

    #[test]
    fn invokes_known_actions_and_ignores_unknown_actions() {
        let mut daemon = NotificationDaemon::default();
        let id = daemon.notify(
            NotificationRequest::new("App", "Ready", "", NotificationUrgency::Low)
                .with_action(NotificationAction::new("default", "Open")),
        );

        assert!(daemon.invoke_action(id, "default"));
        assert!(!daemon.invoke_action(id, "missing"));
        assert_eq!(daemon.actions_invoked().len(), 1);
    }
}
