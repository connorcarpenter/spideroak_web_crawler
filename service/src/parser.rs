use std::str::Chars;

pub(crate) fn find_anchors(html: &str, index: usize, max_index: usize) -> AnchorHrefIterator {
    AnchorHrefIterator::new(html, index, max_index)
}

pub(crate) struct AnchorHrefIterator<'a> {
    html: Chars<'a>,
    index: usize,
    max_index: usize,
    anchor_tag_counter: usize,
    in_tag: bool,
    in_anchor_tag_text: bool,
    has_tag_name: bool,
    tag_name: String,
    current_attr: String,
    current_value: String,
    is_in_href: bool,
    is_in_value: bool,
    quote_char: Option<char>,
    pending_href: Option<String>,
}

impl<'a> AnchorHrefIterator<'a> {
    fn new(html: &'a str, index: usize, max_index: usize) -> Self {
        Self {
            html: html.chars(),
            index,
            max_index,
            anchor_tag_counter: 0,
            in_tag: false,
            in_anchor_tag_text: false,
            has_tag_name: false,
            tag_name: String::new(),
            current_attr: String::new(),
            current_value: String::new(),
            is_in_href: false,
            is_in_value: false,
            quote_char: None,
            pending_href: None,
        }
    }
}

impl<'a> Iterator for AnchorHrefIterator<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(c) = self.html.next() {
            match c {
                '<' => {
                    self.in_tag = true;
                    self.tag_name.clear();
                    self.has_tag_name = false;
                    self.current_attr.clear();
                    self.current_value.clear();
                    self.is_in_href = false;
                    self.is_in_value = false;
                    self.quote_char = None;
                }
                '>' => {
                    if !self.in_tag {
                        continue;
                    }
                    self.in_tag = false;
                    // Handle opening and closing tags
                    if self.tag_name == "a" {
                        // Opening tag
                        self.in_anchor_tag_text = true;
                    } else if self.tag_name.starts_with('/') {
                        // Closing tag
                        let closing_tag_name = self.tag_name.trim_start_matches('/');
                        if closing_tag_name == "a" && self.in_anchor_tag_text {
                            self.in_anchor_tag_text = false;
                            // Increment the anchor tag counter when we close an <a> tag
                            if let Some(href) = self.pending_href.take() {
                                if (self.anchor_tag_counter % self.max_index) == self.index {
                                    self.anchor_tag_counter += 1;
                                    return Some(href);
                                }
                                self.anchor_tag_counter += 1;
                            }
                        }
                    }
                }
                c => {
                    if !self.in_tag {
                        continue;
                    }
                    if !self.has_tag_name {
                        if c.is_whitespace() {
                            if self.tag_name.len() > 0 {
                                self.has_tag_name = true;
                            }
                        } else {
                            self.tag_name.push(c);
                        }
                    } else if self.tag_name != "a" {
                        // skip the remainder of this tag
                    } else if !self.is_in_value {
                        if c.is_whitespace() {
                            // skip whitespace
                        } else if c == '=' {
                            self.is_in_value = true;
                            self.current_value.clear();
                            self.quote_char = None;

                            // Check if the current attribute is "href"
                            if self.current_attr == "href" {
                                self.is_in_href = true;
                            }
                        } else if c != '/' {
                            self.current_attr.push(c);
                        }
                    } else if self.is_in_value {
                        if self.quote_char.is_none() {
                            if c == '"' || c == '\'' {
                                self.quote_char = Some(c);
                            }
                        } else if Some(c) == self.quote_char {
                            // End of attribute value

                            if self.is_in_href {
                                // Store the href to yield after closing tag
                                self.pending_href = Some(self.current_value.clone());
                            }

                            self.is_in_value = false;
                            self.is_in_href = false;
                            self.current_attr.clear();
                            self.current_value.clear();
                            self.quote_char = None;
                        } else {
                            // Inside quoted value
                            self.current_value.push(c);
                        }
                    }
                }
            }
        }
        None
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_case() {
        let html = r#"
            <a href="e">E</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["e",]);
    }

    #[test]
    fn test_empty_html() {
        let html = "";
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert!(hrefs.is_empty());
    }

    #[test]
    fn test_no_anchor_tags() {
        let html = r#"
            <div>No anchor tags here</div>
            <p>Just some text</p>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert!(hrefs.is_empty());
    }

    #[test]
    fn test_malformed_html_missing_closing_tag() {
        let html = r#"
            <a href="https://example.com">Example
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        // The parser should not yield the href since there's no closing </a> tag
        assert!(hrefs.is_empty());
    }

    #[test]
    fn test_different_index_and_max_index() {
        let html = r#"
            <a href="https://example1.com">Example 1</a>
            <a href="https://example2.com">Example 2</a>
            <a href="https://example3.com">Example 3</a>
            <a href="https://example4.com">Example 4</a>
        "#;

        // Worker 0
        let index = 0;
        let max_index = 2;
        let hrefs_worker_0: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(
            hrefs_worker_0,
            vec!["https://example1.com", "https://example3.com"]
        );

        // Worker 1
        let index = 1;
        let hrefs_worker_1: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(
            hrefs_worker_1,
            vec!["https://example2.com", "https://example4.com"]
        );
    }

    #[test]
    fn test_attributes_in_different_order() {
        let html = r#"
            <a id="link1" href="https://example.com">Example</a>
            <a href="https://example.org" class="external">Example Org</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["https://example.com", "https://example.org"]);
    }

    #[test]
    fn test_anchor_tags_without_href() {
        let html = r#"
            <a>Missing href</a>
            <a href="https://example.com">Valid Link</a>
            <a>No href again</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["https://example.com"]);
    }

    #[test]
    fn test_nested_anchor_tags() {
        let html = r#"
            <div>
                <a href="https://example.com">
                    <span>Example</span>
                </a>
            </div>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["https://example.com"]);
    }

    #[test]
    fn test_anchor_tags_with_single_quotes() {
        let html = r#"
            <a href='https://example.com'>Example</a>
            <a href='https://example.org'>Example Org</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["https://example.com", "https://example.org"]);
    }

    #[test]
    fn test_large_html() {
        let mut html = String::new();
        for i in 1..=1000 {
            html.push_str(&format!(
                r#"<a href="https://example{}.com">Example {}</a>"#,
                i, i
            ));
        }
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(&html, index, max_index).collect();
        assert_eq!(hrefs.len(), 1000);
        assert_eq!(hrefs[0], "https://example1.com");
        assert_eq!(hrefs[999], "https://example1000.com");
    }

    #[test]
    fn test_unicode_in_href() {
        let html = r#"
            <a href="https://пример.рф">Unicode Domain</a>
            <a href="https://example.com/路径">Unicode Path</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["https://пример.рф", "https://example.com/路径"]);
    }

    #[test]
    fn test_special_characters_in_attributes() {
        let html = r#"
            <a href="https://example.com?param=1&other=2">Example</a>
            <a href="https://example.org/#fragment">Example Org</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(
            hrefs,
            vec![
                "https://example.com?param=1&other=2",
                "https://example.org/#fragment"
            ]
        );
    }

    #[test]
    fn test_multiple_attributes_before_href() {
        let html = r#"
            <a class="link" data-id="123" href="https://example.com">Example</a>
            <a id="link2" href="https://example.org" title="Example Org">Example Org</a>
        "#;
        let index = 0;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert_eq!(hrefs, vec!["https://example.com", "https://example.org"]);
    }

    #[test]
    fn test_max_index_greater_than_number_of_anchors() {
        let html = r#"
            <a href="https://example1.com">Example 1</a>
            <a href="https://example2.com">Example 2</a>
        "#;
        let index = 0;
        let max_index = 5; // Greater than the number of anchors
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        // Only process anchor tags where (anchor_tag_counter % 5) == 0
        assert_eq!(hrefs, vec!["https://example1.com"]);
    }

    #[test]
    fn test_zero_max_index() {
        let html = r#"
            <a href="https://example.com">Example</a>
        "#;
        let index = 0;
        let max_index = 0;
        // Should handle division by zero or invalid max_index gracefully
        let result = std::panic::catch_unwind(|| {
            let _hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_index() {
        let html = r#"
            <a href="https://example.com">Example</a>
        "#;
        // Since usize cannot be negative, we'll test with an invalid index
        let index = usize::MAX;
        let max_index = 1;
        let hrefs: Vec<String> = find_anchors(html, index, max_index).collect();
        assert!(hrefs.is_empty());
    }
}
