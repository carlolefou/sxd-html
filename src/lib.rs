#![deny(rust_2018_idioms)]

use html5ever::{
    interface::QualName,
    tree_builder::{NextParserState, TreeSink},
    ParseOpts,
};
use html5ever::{tendril::*, tree_builder::NodeOrText};
use sxd_document::{
    dom::{ChildOfElement, Document},
    Package,
};

#[derive(Clone, Debug)]
struct Element<'d> {
    sxd: sxd_document::dom::Element<'d>,
    name: QualName,
}

#[derive(Clone, Debug)]
enum Handle<'d> {
    Doc(Document<'d>),
    Element(Element<'d>),
    Comment(sxd_document::dom::Comment<'d>),
}

#[derive(Clone)]
struct DomBuilder<'d> {
    doc: Document<'d>,
}

impl<'d> TreeSink for DomBuilder<'d> {
    type Handle = Handle<'d>;
    type Output = Handle<'d>;

    fn finish(self) -> Self::Output {
        Handle::Doc(self.doc)
    }

    fn parse_error(&mut self, _msg: std::borrow::Cow<'static, str>) {}

    fn get_document(&mut self) -> Self::Handle {
        Handle::Doc(self.doc)
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> html5ever::ExpandedName<'a> {
        match target {
            Handle::Element(e) => e.name.expanded(),
            _ => unreachable!(),
        }
    }

    fn create_element(
        &mut self,
        name: QualName,
        attrs: Vec<html5ever::Attribute>,
        _flags: html5ever::tree_builder::ElementFlags,
    ) -> Self::Handle {
        // TODO: figure out namespaces
        // TODO: .to_string().as_str() seems bad
        let sxd = self.doc.create_element(name.local.to_string().as_str());
        for attribute in attrs {
            sxd.set_attribute_value(
                attribute.name.local.to_string().as_str(),
                attribute.value.to_string().as_str(),
            );
        }

        Handle::Element(Element { sxd, name })
    }

    fn create_comment(&mut self, text: StrTendril) -> Self::Handle {
        let node = self.doc.create_comment(text.to_string().as_str());
        Handle::Comment(node)
    }

    fn create_pi(&mut self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        unimplemented!();
    }

    fn append(&mut self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(child_handle) => match parent {
                Handle::Doc(doc) => match child_handle {
                    Handle::Comment(child_element) => doc.root().append_child(child_element),
                    Handle::Element(child_element) => doc.root().append_child(child_element.sxd),
                    _ => unreachable!(),
                },
                Handle::Element(parent_element) => match child_handle {
                    Handle::Comment(child_element) => {
                        parent_element.sxd.append_child(child_element)
                    }
                    Handle::Element(child_element) => {
                        parent_element.sxd.append_child(child_element.sxd)
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            NodeOrText::AppendText(new_text) => match parent {
                Handle::Element(parent_element) => {
                    let new_text = new_text.to_string();
                    let children = parent_element.sxd.children();
                    if let Some(ChildOfElement::Text(text_node)) = children.last() {
                        let mut text = text_node.text().to_owned();
                        text.push_str(&new_text);
                        text_node.set_text(&text);
                    } else {
                        parent_element
                            .sxd
                            .append_child(self.doc.create_text(&new_text));
                    }
                }
                _ => unreachable!(),
            },
        };
    }

    fn append_based_on_parent_node(
        &mut self,
        _element: &Self::Handle,
        _prev_element: &Self::Handle,
        _child: NodeOrText<Self::Handle>,
    ) {
        unimplemented!();
    }

    fn append_doctype_to_document(
        &mut self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        // sxd-document seems to ignore Token::DocumentTypeDeclaration
    }

    fn get_template_contents(&mut self, _target: &Self::Handle) -> Self::Handle {
        unimplemented!();
    }

    fn same_node(&self, _x: &Self::Handle, _y: &Self::Handle) -> bool {
        unimplemented!();
    }

    fn set_quirks_mode(&mut self, _mode: html5ever::tree_builder::QuirksMode) {}

    fn append_before_sibling(
        &mut self,
        _sibling: &Self::Handle,
        _new_node: NodeOrText<Self::Handle>,
    ) {
        unimplemented!();
    }

    fn add_attrs_if_missing(&mut self, _target: &Self::Handle, _attrs: Vec<html5ever::Attribute>) {
        unimplemented!();
    }

    fn remove_from_parent(&mut self, _target: &Self::Handle) {
        unimplemented!();
    }

    fn reparent_children(&mut self, _node: &Self::Handle, _new_parent: &Self::Handle) {
        unimplemented!();
    }

    fn mark_script_already_started(&mut self, _node: &Self::Handle) {}

    fn pop(&mut self, _node: &Self::Handle) {}

    fn associate_with_form(
        &mut self,
        _target: &Self::Handle,
        _form: &Self::Handle,
        _nodes: (&Self::Handle, Option<&Self::Handle>),
    ) {
    }

    fn set_current_line(&mut self, _line_number: u64) {}

    fn complete_script(&mut self, _node: &Self::Handle) -> NextParserState {
        NextParserState::Continue
    }
}

pub fn parse(html: &str) -> Package {
    let package = Package::new();
    let doc = package.as_document();
    html5ever::parse_document(DomBuilder { doc }, ParseOpts::default()).one(html);
    package
}

#[cfg(test)]
mod tests {
    use sxd_document::Package;
    use sxd_xpath::Value;

    use super::parse;

    fn get_xpath_result<'a>(package: &'a Package, xpath: &str) -> Value<'a> {
        let factory = sxd_xpath::Factory::new();
        let xpath = factory.build(xpath).expect("Could not compile XPath");
        let xpath = xpath.expect("No XPath was compiled");
        let context = sxd_xpath::Context::new();
        xpath
            .evaluate(&context, package.as_document().root())
            .expect("XPath evaluation failed")
    }

    #[test]
    fn joins_adjacent_text_nodes() {
        let html = "<p>a line\n \nanother line\n</p>";

        let package = parse(html);

        let result = get_xpath_result(&package, "//p/text()");
        assert_eq!(result.string(), "a line\n \nanother line\n");
    }

    #[test]
    fn handles_unclosed_tags() {
        let html = "<html><body><p><br><span>found me!</span></body></html>";

        let package = parse(html);

        let result = get_xpath_result(&package, "/html/body/p/span/text()");
        assert_eq!(result.string(), "found me!");
    }

    #[test]
    fn adds_attributes_to_nodes() {
        let html = "<html><body><div id=\"my_div\" data-attr=\"value\"></div></body></html>";

        let package = parse(html);

        let result = get_xpath_result(&package, "//div[@id=\"my_div\"]/@data-attr");
        assert_eq!(result.string(), "value");
    }

    #[test]
    fn adds_comments_to_dom() {
        let html = "<!--doc comment--><html><body><!--capture the flag--></body></html>";

        let package = parse(html);

        let mut comments = Vec::new();
        if let Value::Nodeset(nodes) = get_xpath_result(&package, "//comment()") {
            for node in nodes.document_order() {
                comments.push(node.comment().unwrap().text());
            }
        }
        assert_eq!(comments, vec!["doc comment", "capture the flag"]);
    }

    #[test]
    fn ignores_doctype() {
        let html = "<!DOCTYPE html><html></html>";
        parse(html);
    }
}
