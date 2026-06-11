//! Entry filtering and predicate logic.

use regex::Regex;

use crate::har::types::Entry;

/// A set of optional filters applied to HAR entries.
///
/// All set filters must match for an entry to pass (AND logic).
/// Unset filters (None) match everything.
#[derive(Debug, Clone)]
pub struct EntryPredicate {
    url_regex: Option<Regex>,
    status: Option<u16>,
    method: Option<String>,
    mime_type: Option<String>,
    domain: Option<String>,
}

impl EntryPredicate {
    /// Create a new predicate with no filters (matches everything).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            url_regex: None,
            status: None,
            method: None,
            mime_type: None,
            domain: None,
        }
    }

    /// Filter entries whose URL matches a regex pattern.
    ///
    /// # Errors
    ///
    /// Returns an error if the regex pattern is invalid.
    pub fn with_url_pattern(
        mut self,
        pattern: &str,
    ) -> Result<Self, regex::Error> {
        self.url_regex = Some(Regex::new(pattern)?);
        Ok(self)
    }

    /// Filter by exact HTTP status code.
    #[must_use]
    pub const fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by exact HTTP method (e.g. "GET", "POST").
    #[must_use]
    pub fn with_method(mut self, method: &str) -> Self {
        self.method = Some(method.to_uppercase());
        self
    }

    /// Filter by response MIME type (e.g. "application/json").
    #[must_use]
    pub fn with_mime_type(mut self, mime_type: &str) -> Self {
        self.mime_type = Some(mime_type.to_ascii_lowercase());
        self
    }

    /// Filter by domain (exact match on the host portion of the URL).
    #[must_use]
    pub fn with_domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_ascii_lowercase());
        self
    }

    /// Check whether an entry matches all set filters.
    #[must_use]
    pub fn matches(&self, entry: &Entry) -> bool {
        if let Some(ref regex) = self.url_regex {
            if !regex.is_match(&entry.request.url) {
                return false;
            }
        }

        if let Some(status) = self.status {
            if entry.response.status != status {
                return false;
            }
        }

        if let Some(ref method) = self.method {
            if !entry.request.method.eq_ignore_ascii_case(method) {
                return false;
            }
        }

        if let Some(ref mime_type) = self.mime_type {
            if !entry
                .response
                .content
                .mime_type
                .eq_ignore_ascii_case(mime_type)
            {
                return false;
            }
        }

        if let Some(ref domain) = self.domain {
            let entry_domain = crate::har::stats::extract_domain(entry);
            if !entry_domain.eq_ignore_ascii_case(domain) {
                return false;
            }
        }

        true
    }
}

impl Default for EntryPredicate {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter entries using the given predicate, returning references to
/// matching entries in the same order they appear in the log.
#[must_use]
pub fn filter_entries<'a>(
    entries: &'a [Entry],
    predicate: &EntryPredicate,
) -> Vec<&'a Entry> {
    entries.iter().filter(|e| predicate.matches(e)).collect()
}

#[cfg(test)]
#[path = "tests/filter.rs"]
mod tests;
