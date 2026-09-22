use super::vault::{BrowsePage, Page, Vault};

impl Vault {
    pub(crate) fn toggle_structure(&mut self) {
        self.hide();
        self.page = match self.page {
            Page::Browse(BrowsePage::Tree) => Page::Browse(BrowsePage::Entries),
            Page::Browse(BrowsePage::Entries) => Page::Browse(BrowsePage::Tree),
            Page::Password(page) | Page::DeleteConfirm(page) => Page::Browse(page),
        };
    }

    pub(crate) fn is_tree(&self) -> bool {
        matches!(self.page, Page::Browse(BrowsePage::Tree))
    }

    pub(crate) fn show_tree(&mut self) {
        self.hide();
        self.page = Page::Browse(BrowsePage::Tree);
    }

    pub(crate) fn open_password(&mut self) {
        if let Page::Browse(page) = self.page {
            self.page = Page::Password(page);
        }
    }

    pub(crate) fn request_delete(&mut self) {
        if let Page::Browse(page) = self.page {
            self.page = Page::DeleteConfirm(page);
        }
    }

    pub(crate) fn close_dialog(&mut self) {
        self.page = match self.page {
            Page::Password(page) | Page::DeleteConfirm(page) => Page::Browse(page),
            page => page,
        };
    }
}
