use std::io::Write;

use liquid_core::error::ResultLiquidExt;
use liquid_core::model::{ValueView, ValueViewCmp};
use liquid_core::parser::BlockElement;
use liquid_core::parser::TryMatchToken;
use liquid_core::Expression;
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::Template;
use liquid_core::{BlockReflection, ParseBlock, TagBlock, TagTokenIter};

#[derive(Copy, Clone, Debug, Default)]
pub struct CaseBlock;

impl CaseBlock {
    pub fn new() -> Self {
        Self
    }
}

impl BlockReflection for CaseBlock {
    fn start_tag(&self) -> &str {
        "case"
    }

    fn end_tag(&self) -> &str {
        "endcase"
    }

    fn description(&self) -> &str {
        ""
    }
}


#[derive(Debug)]
struct CaseOptionData {
    condition: Vec<Expression>,
    elements: Vec<Box<dyn Renderable>>,
}

impl ParseBlock for CaseBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut tokens: TagBlock<'_, '_>,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let target = arguments
            .expect_next("Value expected.")?
            .expect_value()
            .into_result()?;

        // no more arguments should be supplied, trying to supply them is an error
        arguments.expect_nothing()?;

        let mut cases = Vec::new();
        let mut cases_data = Vec::new();
        let mut else_block = None;
        let mut current_block = Vec::new();
        let mut current_condition = None;
        let mut is_blank = true;

        while let Some(element) = tokens.next()? {
            match element {
                BlockElement::Tag(mut tag) => match tag.name() {
                    "when" => {
                        if let Some(condition) = current_condition {
                            is_blank &= current_block.iter().all(|x: &Box<dyn Renderable>| x.is_blank());
                            cases_data.push(CaseOptionData{condition, elements: current_block});
                        }
                        current_block = Vec::new();
                        current_condition = Some(parse_condition(tag.tokens())?);
                    }
                    "else" => {
                        // no more arguments should be supplied, trying to supply them is an error
                        tag.tokens().expect_nothing()?;
                        else_block = Some(tokens.parse_all(options)?);
                        break;
                    }
                    _ => current_block.push(tag.parse(&mut tokens, options)?),
                },
                element => current_block.push(element.parse(&mut tokens, options)?),
            }
        }

        if let Some(condition) = current_condition {
            is_blank &= current_block.iter().all(|x: &Box<dyn Renderable>| x.is_blank());
            cases_data.push(CaseOptionData{condition, elements: current_block});
        }

        if let Some(else_blk) = &else_block {
            is_blank &= else_blk.iter().all(|x| x.is_blank());
        };


        // Now filter out the else and cases if is_blank is true
        for data in cases_data {
            let elements = if is_blank {
                data.elements.into_iter().filter(|x| !x.is_text()).collect()
            } else {
                data.elements
            };
            cases.push(CaseOption::new(data.condition, Template::new(elements)));
        }
        if is_blank {
            else_block = else_block.map(|x| x.into_iter().filter(|x| !x.is_text()).collect());
        }
        let else_block = else_block.map(Template::new);

        tokens.assert_empty();
        Ok(Box::new(Case {
            target,
            is_blank,
            cases,
            else_block,
        }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

fn parse_condition(arguments: &mut TagTokenIter<'_>) -> Result<Vec<Expression>> {
    let mut values = Vec::new();

    let first_value = arguments
        .expect_next("Value expected")?
        .expect_value()
        .into_result()?;
    values.push(first_value);

    while let Some(token) = arguments.next() {
        if let TryMatchToken::Fails(token) = token.expect_case_insensitive_str("or") {
            token
                .expect_str(",")
                .into_result_custom_msg("\"or\" or \",\" expected.")?;
        }

        let value = arguments
            .expect_next("Value expected")?
            .expect_value()
            .into_result()?;
        values.push(value);
    }

    // no more arguments should be supplied, trying to supply them is an error
    arguments.expect_nothing()?;
    Ok(values)
}

#[derive(Debug)]
struct Case {
    is_blank: bool,
    target: Expression,
    cases: Vec<CaseOption>,
    else_block: Option<Template>,
}

impl Case {
    fn trace(&self) -> String {
        format!("{{% case {} %}}", self.target)
    }
}

impl Renderable for Case {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()> {
        let value = self.target.evaluate(runtime)?.to_value();
        for case in &self.cases {
            if case.evaluate(&value, runtime)? {
                return case
                    .template
                    .render_to(writer, runtime)
                    .trace_with(|| case.trace().into())
                    .trace_with(|| self.trace().into())
                    .context_key_with(|| self.target.to_string().into())
                    .value_with(|| value.to_kstr().into_owned());
            }
        }

        if let Some(ref t) = self.else_block {
            return t
                .render_to(writer, runtime)
                .trace("{{% else %}}")
                .trace_with(|| self.trace().into())
                .context_key_with(|| self.target.to_string().into())
                .value_with(|| value.to_kstr().into_owned());
        }

        Ok(())
    }

    fn is_blank(&self) -> bool {
        self.is_blank
    }

}

#[derive(Debug)]
struct CaseOption {
    args: Vec<Expression>,
    template: Template,
}

impl CaseOption {
    fn new(args: Vec<Expression>, template: Template) -> CaseOption {
        CaseOption { args, template }
    }

    fn evaluate(&self, value: &dyn ValueView, runtime: &dyn Runtime) -> Result<bool> {
        for a in &self.args {
            let v = a.evaluate(runtime)?;
            if v == ValueViewCmp::new(value) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn trace(&self) -> String {
        format!("{{% when {} %}}", itertools::join(self.args.iter(), " or "))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use liquid_core::model::Value;
    use liquid_core::parser;
    use liquid_core::runtime;
    use liquid_core::runtime::RuntimeBuilder;

    fn options() -> Language {
        let mut options = Language::default();
        options
            .blocks
            .register("case".to_string(), CaseBlock.into());
        options
            .tags
            .register("assign".to_string(), crate::stdlib::AssignTag.into());
        options
    }

    #[test]
    fn test_case_block() {
        let text = concat!(
            "{% case x %}",
            "{% when 2 %}",
            "two",
            "{% when 3 or 4 %}",
            "three and a half",
            "{% else %}",
            "otherwise",
            "{% endcase %}"
        );
        let options = options();
        let template = parser::parse(text, &options)
            .map(runtime::Template::new)
            .unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar(2f64));
        assert_eq!(template.render(&runtime).unwrap(), "two");

        runtime.set_global("x".into(), Value::scalar(3f64));
        assert_eq!(template.render(&runtime).unwrap(), "three and a half");

        runtime.set_global("x".into(), Value::scalar(4f64));
        assert_eq!(template.render(&runtime).unwrap(), "three and a half");

        runtime.set_global("x".into(), Value::scalar("nope"));
        assert_eq!(template.render(&runtime).unwrap(), "otherwise");
    }

    #[test]
    fn test_no_matches_returns_empty_string() {
        let text = concat!(
            "{% case x %}",
            "{% when 2 %}",
            "two",
            "{% when 3 or 4 %}",
            "three and a half",
            "{% endcase %}"
        );
        let options = options();
        let template = parser::parse(text, &options)
            .map(runtime::Template::new)
            .unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar("nope"));
        assert_eq!(template.render(&runtime).unwrap(), "");
    }

    #[test]
    fn multiple_else_blocks_is_an_error() {
        let text = concat!(
            "{% case x %}",
            "{% when 2 %}",
            "two",
            "{% else %}",
            "else #1",
            "{% else %}",
            "else # 2",
            "{% endcase %}"
        );
        let options = options();
        let template = parser::parse(text, &options).map(runtime::Template::new);
        assert!(template.is_err());
    }


    #[test]
    fn remove_whitespaces() {
        let text = r#"
            {%- case x %}
            {% when 1 %}
                {% assign var=1 %}
            {% when 2 %}
                {% assign var=2 %}
            {% when 3 %}
                {% assign var=3 %}
            {% when 4 %}
                {% assign var=4 %}
            {% when 5 %}
                {% assign var=5 %}
            {% else %}
                {% assign var=0 %}
            {% endcase -%}
        "#;
        let options = options();
        let template = parser::parse(text, &options).map(runtime::Template::new).unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar(1));
        assert_eq!(template.render(&runtime).unwrap(), "");

        let text = r#"
            {%- case x %}
            {% when 1 %}
                {% case y %}
                {% when 1 %}
                    {% assign var=1 %}
                {% when 2 %}
                    {% assign var=2 %}
                {% endcase %}
            {% when 2 %}
                {% assign var=2 %}
            {% when 3 %}
                {% assign var=3 %}
            {% when 4 %}
                {% assign var=4 %}
            {% when 5 %}
                {% assign var=5 %}
            {% else %}
                 {% assign var=0 %}
            {% endcase -%}
        "#;
        let template = parser::parse(text, &options).map(runtime::Template::new).unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar(1));
        runtime.set_global("y".into(), Value::scalar(1));
        assert_eq!(template.render(&runtime).unwrap(), "");
    }


    #[test]
    fn will_not_remove_whitespaces() {
        let text = r#"
            {%- case x %}
            {% when 1 %}
                {% assign var=1 %}
            {% when 2 %}
                {% assign var=2 %}
            {% when 3 %}
                {% assign var=3 %}
                single nonempty section
            {% when 4 %}
                {% assign var=4 %}
            {% when 5 %}
                {% assign var=5 %}
            {% else %}
                 {% assign var=0 %}
            {% endcase -%}
        "#;
        let options = options();
        let template = parser::parse(text, &options).map(runtime::Template::new).unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar(1));
        assert_ne!(template.render(&runtime).unwrap(), "");

        let text = r#"
            {%- case x %}
            {% when 1 %}
                {% case y %}
                {% when 1 %}
                    {% assign var=1 %}
                {% when 2 %}
                    {% assign var=2 %}
                {% else %}
                    single nonempty section
                {% endcase %}
            {% when 2 %}
                {% assign var=2 %}
            {% when 3 %}
                {% assign var=3 %}
            {% when 4 %}
                {% assign var=4 %}
            {% when 5 %}
                {% assign var=5 %}
            {% else %}
                 {% assign var=0 %}
            {% endcase -%}
        "#;
        let template = parser::parse(text, &options).map(runtime::Template::new).unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar(1));
        runtime.set_global("y".into(), Value::scalar(1));
        assert_ne!(template.render(&runtime).unwrap(), "");
    }

}
