//! Where an action's result is shown: on the page where the action was taken, until the user
//! dismisses it or takes another action. Moving between pages neither shows it elsewhere nor
//! clears it.

use crate::model::Page;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notices {
    current: Option<(Page, String)>,
    /// The page the latest group action was taken on; its result belongs there.
    action_page: Option<Page>,
}

impl Notices {
    pub fn show(&mut self, page: Page, text: impl Into<String>) {
        self.current = Some((page, text.into()));
    }

    /// A group action is being sent from `page`.
    pub fn action_started(&mut self, page: Page) {
        self.current = None;
        self.action_page = Some(page);
    }

    /// The result of the latest group action. It stays with the page that sent it.
    pub fn result(&mut self, fallback: Page, text: impl Into<String>) {
        let page = self.action_page.unwrap_or(fallback);
        self.show(page, text);
    }

    pub fn for_page(&self, page: Page) -> Option<&str> {
        match &self.current {
            Some((p, t)) if *p == page => Some(t),
            _ => None,
        }
    }

    pub fn dismiss(&mut self) {
        self.current = None;
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.action_page = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_shows_on_the_page_of_the_action_and_survives_moving_around() {
        let mut n = Notices::default();
        n.action_started(Page::Addons);
        n.result(Page::Overview, "That isn't available.");
        assert_eq!(n.for_page(Page::Addons), Some("That isn't available."));
        assert_eq!(n.for_page(Page::Overview), None);
        assert_eq!(n.for_page(Page::Addons), Some("That isn't available."));
    }

    #[test]
    fn it_goes_away_on_dismiss_or_the_next_action_and_not_before() {
        let mut n = Notices::default();
        n.show(Page::Computers, "Saved");
        assert!(n.for_page(Page::Computers).is_some());
        n.action_started(Page::Computers);
        assert_eq!(n.for_page(Page::Computers), None);
        n.result(Page::Computers, "Done");
        n.dismiss();
        assert_eq!(n.for_page(Page::Computers), None);
    }

    #[test]
    fn a_result_with_no_known_action_page_uses_the_current_page() {
        let mut n = Notices::default();
        n.result(Page::About, "Checked");
        assert_eq!(n.for_page(Page::About), Some("Checked"));
    }
}
