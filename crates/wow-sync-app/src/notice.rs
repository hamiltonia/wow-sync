//! Where an action's result is shown: on the page where the action was taken, until the user
//! dismisses it or takes another action. Moving between pages neither shows it elsewhere nor
//! clears it.

use crate::model::Page;
use wow_sync_core::ids::Sha256;

/// A rollback request a notice is about: the add-on key and the release asked for.
pub type RollbackScope = (String, Sha256);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notices {
    current: Option<(Page, String, Option<RollbackScope>)>,
    /// The page the latest group action was taken on; its result belongs there.
    action_page: Option<Page>,
    /// The rollback the latest group action asked for, if it was one.
    action_scope: Option<RollbackScope>,
}

impl Notices {
    pub fn show(&mut self, page: Page, text: impl Into<String>) {
        self.current = Some((page, text.into(), None));
    }

    /// A group action is being sent from `page`; `scope` is set when it is a rollback.
    pub fn action_started(&mut self, page: Page, scope: Option<RollbackScope>) {
        self.current = None;
        self.action_page = Some(page);
        self.action_scope = scope;
    }

    /// The result of the latest group action. It stays with the page that sent it, and with
    /// its add-on when it was a rollback.
    pub fn result(&mut self, fallback: Page, text: impl Into<String>) {
        let page = self.action_page.unwrap_or(fallback);
        self.current = Some((page, text.into(), self.action_scope.clone()));
    }

    /// The notice for `page`; a rollback notice shows only on its own add-on's page.
    pub fn for_page(&self, page: Page, addon: Option<&str>) -> Option<&str> {
        match &self.current {
            Some((p, t, scope)) if *p == page => match scope {
                Some((key, _)) if addon != Some(key.as_str()) => None,
                _ => Some(t),
            },
            _ => None,
        }
    }

    /// Drop a rollback notice once `done(add-on, release)` says the request is finished.
    pub fn settle(&mut self, done: impl Fn(&str, &Sha256) -> bool) {
        if let Some((_, _, Some((key, release)))) = &self.current {
            if done(key, release) {
                self.current = None;
            }
        }
    }

    pub fn dismiss(&mut self) {
        self.current = None;
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.action_page = None;
        self.action_scope = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_shows_on_the_page_of_the_action_and_survives_moving_around() {
        let mut n = Notices::default();
        n.action_started(Page::Addons, None);
        n.result(Page::Overview, "That isn't available.");
        assert_eq!(
            n.for_page(Page::Addons, None),
            Some("That isn't available.")
        );
        assert_eq!(n.for_page(Page::Overview, None), None);
        assert_eq!(
            n.for_page(Page::Addons, None),
            Some("That isn't available.")
        );
    }

    #[test]
    fn it_goes_away_on_dismiss_or_the_next_action_and_not_before() {
        let mut n = Notices::default();
        n.show(Page::Computers, "Saved");
        assert!(n.for_page(Page::Computers, None).is_some());
        n.action_started(Page::Computers, None);
        assert_eq!(n.for_page(Page::Computers, None), None);
        n.result(Page::Computers, "Done");
        n.dismiss();
        assert_eq!(n.for_page(Page::Computers, None), None);
    }

    #[test]
    fn a_result_with_no_known_action_page_uses_the_current_page() {
        let mut n = Notices::default();
        n.result(Page::About, "Checked");
        assert_eq!(n.for_page(Page::About, None), Some("Checked"));
    }

    #[test]
    fn a_rollback_notice_stays_on_its_add_on_and_settles_when_done() {
        let rel = Sha256([1; 32]);
        let mut n = Notices::default();
        n.action_started(Page::Addons, Some(("beta".into(), rel)));
        n.result(Page::Addons, "Requested.");
        assert_eq!(n.for_page(Page::Addons, Some("beta")), Some("Requested."));
        assert_eq!(n.for_page(Page::Addons, Some("alpha")), None);
        assert_eq!(n.for_page(Page::Addons, None), None);
        n.settle(|_, _| false);
        assert!(n.for_page(Page::Addons, Some("beta")).is_some());
        n.settle(|k, r| k == "beta" && *r == rel);
        assert_eq!(n.for_page(Page::Addons, Some("beta")), None);
    }
}
