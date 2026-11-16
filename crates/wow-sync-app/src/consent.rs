//! Consent checkbox state. Consent is never carried over between panels or previews.

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Consent {
    pub identity: bool,
    pub scope: bool,
    panel: Option<String>,
}

impl Consent {
    /// Call every render with the identity of the consent panel on screen (`None` when
    /// none is shown). Any change of panel, including it disappearing, clears both boxes.
    pub fn observe(&mut self, panel: Option<String>) {
        if self.panel != panel {
            self.identity = false;
            self.scope = false;
            self.panel = panel;
        }
    }

    /// Consent is single-use: clear it once a request has been sent.
    pub fn clear(&mut self) {
        self.identity = false;
        self.scope = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_reset_when_the_panel_is_reshown_or_changes() {
        let mut c = Consent::default();
        c.observe(Some("reconfirm|a".into()));
        c.identity = true;
        c.scope = true;
        c.observe(Some("reconfirm|a".into()));
        assert!(c.identity && c.scope, "same panel keeps the user's ticks");
        c.observe(Some("reconfirm|b".into()));
        assert!(
            !c.identity && !c.scope,
            "changed preview needs fresh consent"
        );
        c.identity = true;
        c.observe(None);
        c.observe(Some("reconfirm|b".into()));
        assert!(
            !c.identity && !c.scope,
            "re-shown panel needs fresh consent"
        );
    }

    #[test]
    fn clear_is_single_use() {
        let mut c = Consent::default();
        c.observe(Some("x".into()));
        c.identity = true;
        c.scope = true;
        c.clear();
        assert!(!c.identity && !c.scope);
    }
}
