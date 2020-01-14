use std::fmt;
use std::io::Write;

use liquid_core::compiler::BlockElement;
use liquid_core::compiler::TagToken;
use liquid_core::error::ResultLiquidExt;
use liquid_core::value::{Value, ValueView, ValueViewCmp};
use liquid_core::Context;
use liquid_core::Expression;
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Template;
use liquid_core::{BlockReflection, ParseBlock, TagBlock, TagTokenIter};
use liquid_core::{Error, Result};

#[derive(Clone, Debug)]
enum ComparisonOperator {
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanEquals,
    GreaterThanEquals,
    Contains,
}

impl fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out = match *self {
            ComparisonOperator::Equals => "==",
            ComparisonOperator::NotEquals => "!=",
            ComparisonOperator::LessThanEquals => "<=",
            ComparisonOperator::GreaterThanEquals => ">=",
            ComparisonOperator::LessThan => "<",
            ComparisonOperator::GreaterThan => ">",
            ComparisonOperator::Contains => "contains",
        };
        write!(f, "{}", out)
    }
}

impl ComparisonOperator {
    fn from_str(s: &str) -> ::std::result::Result<Self, ()> {
        match s {
            "==" => Ok(ComparisonOperator::Equals),
            "!=" | "<>" => Ok(ComparisonOperator::NotEquals),
            "<" => Ok(ComparisonOperator::LessThan),
            ">" => Ok(ComparisonOperator::GreaterThan),
            "<=" => Ok(ComparisonOperator::LessThanEquals),
            ">=" => Ok(ComparisonOperator::GreaterThanEquals),
            "contains" => Ok(ComparisonOperator::Contains),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
struct BinaryCondition {
    lh: Expression,
    comparison: ComparisonOperator,
    rh: Expression,
}

impl BinaryCondition {
    pub fn evaluate(&self, context: &Context) -> Result<bool> {
        let a = self.lh.evaluate(context)?;
        let ca = ValueViewCmp::new(a);
        let b = self.rh.evaluate(context)?;
        let cb = ValueViewCmp::new(b);

        let result = match self.comparison {
            ComparisonOperator::Equals => ca == cb,
            ComparisonOperator::NotEquals => ca != cb,
            ComparisonOperator::LessThan => ca < cb,
            ComparisonOperator::GreaterThan => ca > cb,
            ComparisonOperator::LessThanEquals => ca <= cb,
            ComparisonOperator::GreaterThanEquals => ca >= cb,
            ComparisonOperator::Contains => contains_check(a, b)?,
        };

        Ok(result)
    }
}

impl fmt::Display for BinaryCondition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.lh, self.comparison, self.rh)
    }
}

#[derive(Clone, Debug)]
struct ExistenceCondition {
    lh: Expression,
}

static NIL: Value = Value::Nil;

impl ExistenceCondition {
    pub fn evaluate(&self, context: &Context) -> Result<bool> {
        let a = self.lh.try_evaluate(context).unwrap_or(&NIL);
        Ok(a.query_state(liquid_core::value::State::Truthy))
    }
}

impl fmt::Display for ExistenceCondition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.lh)
    }
}

#[derive(Clone, Debug)]
enum Condition {
    Binary(BinaryCondition),
    Existence(ExistenceCondition),
    Conjunction(Box<Condition>, Box<Condition>),
    Disjunction(Box<Condition>, Box<Condition>),
}

impl Condition {
    pub fn evaluate(&self, context: &Context) -> Result<bool> {
        match *self {
            Condition::Binary(ref c) => c.evaluate(context),
            Condition::Existence(ref c) => c.evaluate(context),
            Condition::Conjunction(ref left, ref right) => {
                Ok(left.evaluate(context)? && right.evaluate(context)?)
            }
            Condition::Disjunction(ref left, ref right) => {
                Ok(left.evaluate(context)? || right.evaluate(context)?)
            }
        }
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Condition::Binary(ref c) => write!(f, "{}", c),
            Condition::Existence(ref c) => write!(f, "{}", c),
            Condition::Conjunction(ref left, ref right) => write!(f, "{} and {}", left, right),
            Condition::Disjunction(ref left, ref right) => write!(f, "{} or {}", left, right),
        }
    }
}

#[derive(Debug)]
struct Conditional {
    tag_name: String,
    condition: Condition,
    mode: bool,
    if_true: Template,
    if_false: Option<Template>,
}

fn contains_check(a: &dyn ValueView, b: &dyn ValueView) -> Result<bool> {
    if let Some(a) = a.as_scalar() {
        let b = b.to_kstr();
        Ok(a.to_kstr().contains(b.as_str()))
    } else if let Some(a) = a.as_object() {
        let b = b.as_scalar();
        let check = b
            .map(|b| a.contains_key(b.to_kstr().as_str()))
            .unwrap_or(false);
        Ok(check)
    } else if let Some(a) = a.as_array() {
        for elem in a.values() {
            if ValueViewCmp::new(elem) == ValueViewCmp::new(b) {
                return Ok(true);
            }
        }
        Ok(false)
    } else {
        Err(unexpected_value_error(
            "string | array | object",
            Some(a.type_name()),
        ))
    }
}

impl Conditional {
    fn compare(&self, context: &Context) -> Result<bool> {
        let result = self.condition.evaluate(context)?;

        Ok(result == self.mode)
    }

    fn trace(&self) -> String {
        format!("{{% if {} %}}", self.condition)
    }
}

impl Renderable for Conditional {
    fn render_to(&self, writer: &mut dyn Write, context: &mut Context) -> Result<()> {
        let condition = self.compare(context).trace_with(|| self.trace().into())?;
        if condition {
            self.if_true
                .render_to(writer, context)
                .trace_with(|| self.trace().into())?;
        } else if let Some(ref template) = self.if_false {
            template
                .render_to(writer, context)
                .trace("{{% else %}}")
                .trace_with(|| self.trace().into())?;
        }

        Ok(())
    }
}

struct PeekableTagTokenIter<'a> {
    iter: TagTokenIter<'a>,
    peeked: Option<Option<TagToken<'a>>>,
}

impl<'a> Iterator for PeekableTagTokenIter<'a> {
    type Item = TagToken<'a>;

    fn next(&mut self) -> Option<TagToken<'a>> {
        match self.peeked.take() {
            Some(v) => v,
            None => self.iter.next(),
        }
    }
}

impl<'a> PeekableTagTokenIter<'a> {
    pub fn expect_next(&mut self, error_msg: &str) -> Result<TagToken<'a>> {
        self.next().ok_or_else(|| self.iter.raise_error(error_msg))
    }

    fn peek(&mut self) -> Option<&TagToken<'a>> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match self.peeked {
            Some(Some(ref value)) => Some(value),
            Some(None) => None,
            None => unreachable!(),
        }
    }
}

fn parse_atom_condition(arguments: &mut PeekableTagTokenIter) -> Result<Condition> {
    let lh = arguments
        .expect_next("Value expected.")?
        .expect_value()
        .into_result()?;
    let cond = match arguments
        .peek()
        .map(TagToken::as_str)
        .and_then(|op| ComparisonOperator::from_str(op).ok())
    {
        Some(op) => {
            arguments.next();
            let rh = arguments
                .expect_next("Value expected.")?
                .expect_value()
                .into_result()?;
            Condition::Binary(BinaryCondition {
                lh,
                comparison: op,
                rh,
            })
        }
        None => Condition::Existence(ExistenceCondition { lh }),
    };

    Ok(cond)
}

fn parse_conjunction_chain(arguments: &mut PeekableTagTokenIter) -> Result<Condition> {
    let mut lh = parse_atom_condition(arguments)?;

    while let Some("and") = arguments.peek().map(TagToken::as_str) {
        arguments.next();
        let rh = parse_atom_condition(arguments)?;
        lh = Condition::Conjunction(Box::new(lh), Box::new(rh));
    }

    Ok(lh)
}

/// Common parsing for "if" and "unless" condition
fn parse_condition(arguments: TagTokenIter) -> Result<Condition> {
    let mut arguments = PeekableTagTokenIter {
        iter: arguments,
        peeked: None,
    };
    let mut lh = parse_conjunction_chain(&mut arguments)?;

    while let Some(token) = arguments.next() {
        token
            .expect_str("or")
            .into_result_custom_msg("\"and\" or \"or\" expected.")?;

        let rh = parse_conjunction_chain(&mut arguments)?;
        lh = Condition::Disjunction(Box::new(lh), Box::new(rh));
    }

    Ok(lh)
}

#[derive(Copy, Clone, Debug, Default)]
pub struct UnlessBlock;

impl UnlessBlock {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlockReflection for UnlessBlock {
    fn start_tag(&self) -> &str {
        "unless"
    }

    fn end_tag(&self) -> &str {
        "endunless"
    }

    fn description(&self) -> &str {
        ""
    }
}

impl ParseBlock for UnlessBlock {
    fn parse(
        &self,
        arguments: TagTokenIter,
        mut tokens: TagBlock,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let condition = parse_condition(arguments)?;

        let mut if_true = Vec::new();
        let mut if_false = None;

        while let Some(element) = tokens.next()? {
            match element {
                BlockElement::Tag(tag) => match tag.name() {
                    "else" => {
                        if_false = Some(tokens.parse_all(options)?);
                        break;
                    }
                    _ => if_true.push(tag.parse(&mut tokens, options)?),
                },
                element => if_true.push(element.parse(&mut tokens, options)?),
            }
        }

        let if_true = Template::new(if_true);
        let if_false = if_false.map(Template::new);

        tokens.assert_empty();
        Ok(Box::new(Conditional {
            tag_name: self.start_tag().to_string(),
            condition,
            mode: false,
            if_true,
            if_false,
        }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

fn parse_if(
    tag_name: &str,
    arguments: TagTokenIter,
    tokens: &mut TagBlock,
    options: &Language,
) -> Result<Box<dyn Renderable>> {
    let condition = parse_condition(arguments)?;

    let mut if_true = Vec::new();
    let mut if_false = None;

    while let Some(element) = tokens.next()? {
        match element {
            BlockElement::Tag(tag) => match tag.name() {
                "else" => {
                    if_false = Some(tokens.parse_all(options)?);
                    break;
                }
                "elsif" => {
                    if_false = Some(vec![parse_if("elsif", tag.into_tokens(), tokens, options)?]);
                    break;
                }
                _ => if_true.push(tag.parse(tokens, options)?),
            },
            element => if_true.push(element.parse(tokens, options)?),
        }
    }

    let if_true = Template::new(if_true);
    let if_false = if_false.map(Template::new);

    Ok(Box::new(Conditional {
        tag_name: tag_name.to_string(),
        condition,
        mode: true,
        if_true,
        if_false,
    }))
}

#[derive(Copy, Clone, Debug, Default)]
pub struct IfBlock;

impl IfBlock {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlockReflection for IfBlock {
    fn start_tag(&self) -> &str {
        "if"
    }

    fn end_tag(&self) -> &str {
        "endif"
    }

    fn description(&self) -> &str {
        ""
    }
}

impl ParseBlock for IfBlock {
    fn parse(
        &self,
        arguments: TagTokenIter,
        mut tokens: TagBlock,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let conditional = parse_if(self.start_tag(), arguments, &mut tokens, options)?;

        tokens.assert_empty();
        Ok(conditional)
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

/// Format an error for an unexpected value.
pub fn unexpected_value_error<S: ToString>(expected: &str, actual: Option<S>) -> Error {
    let actual = actual.map(|x| x.to_string());
    unexpected_value_error_string(expected, actual)
}

fn unexpected_value_error_string(expected: &str, actual: Option<String>) -> Error {
    let actual = actual.unwrap_or_else(|| "nothing".to_owned());
    Error::with_msg(format!("Expected {}, found `{}`", expected, actual))
}

#[cfg(test)]
mod test {
    use super::*;

    use liquid_core::compiler;
    use liquid_core::interpreter;
    use liquid_core::value::Object;
    use liquid_core::value::Value;

    fn options() -> Language {
        let mut options = Language::default();
        options.blocks.register("if".to_string(), IfBlock.into());
        options
            .blocks
            .register("unless".to_string(), UnlessBlock.into());
        options
    }

    #[test]
    fn number_comparison() {
        let text = "{% if 6 < 7  %}if true{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");

        let text = "{% if 7 < 6  %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn string_comparison() {
        let text = r#"{% if "one" == "one"  %}if true{% endif %}"#;
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");

        let text = r#"{% if "one" == "two"  %}if true{% else %}if false{% endif %}"#;
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn implicit_comparison() {
        let text = concat!(
            "{% if truthy %}",
            "yep",
            "{% else %}",
            "nope",
            "{% endif %}"
        );

        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        // Non-existence
        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "nope");

        // Explicit nil
        let mut context = Context::new();
        context.stack_mut().set_global("truthy", Value::Nil);
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "nope");

        // false
        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("truthy", Value::scalar(false));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "nope");

        // true
        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("truthy", Value::scalar(true));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "yep");
    }

    #[test]
    fn unless() {
        let text = concat!(
            "{% unless some_value == 1 %}",
            "unless body",
            "{% endunless %}"
        );

        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("some_value", Value::scalar(1f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "");

        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("some_value", Value::scalar(42f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "unless body");
    }

    #[test]
    fn nested_if_else() {
        let text = concat!(
            "{% if truthy %}",
            "yep, ",
            "{% if also_truthy %}",
            "also truthy",
            "{% else %}",
            "not also truthy",
            "{% endif %}",
            "{% else %}",
            "nope",
            "{% endif %}"
        );
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("truthy", Value::scalar(true));
        context
            .stack_mut()
            .set_global("also_truthy", Value::scalar(false));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "yep, not also truthy");
    }

    #[test]
    fn multiple_elif_blocks() {
        let text = concat!(
            "{% if a == 1 %}",
            "first",
            "{% elsif a == 2 %}",
            "second",
            "{% elsif a == 3 %}",
            "third",
            "{% else %}",
            "fourth",
            "{% endif %}"
        );

        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        context.stack_mut().set_global("a", Value::scalar(1f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "first");

        let mut context = Context::new();
        context.stack_mut().set_global("a", Value::scalar(2f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "second");

        let mut context = Context::new();
        context.stack_mut().set_global("a", Value::scalar(3f64));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "third");

        let mut context = Context::new();
        context.stack_mut().set_global("a", Value::scalar("else"));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "fourth");
    }

    #[test]
    fn string_contains_with_literals() {
        let text = "{% if \"Star Wars\" contains \"Star\" %}if true{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");

        let text = "{% if \"Star Wars\" contains \"Alf\"  %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn string_contains_with_variables() {
        let text = "{% if movie contains \"Star\"  %}if true{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("movie", Value::scalar("Star Wars"));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");

        let text = "{% if movie contains \"Star\"  %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        context
            .stack_mut()
            .set_global("movie", Value::scalar("Batman"));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn contains_with_object_and_key() {
        let text = "{% if movies contains \"Star Wars\" %}if true{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let mut obj = Object::new();
        obj.insert("Star Wars".into(), Value::scalar("1977"));
        context.stack_mut().set_global("movies", Value::Object(obj));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");
    }

    #[test]
    fn contains_with_object_and_missing_key() {
        let text = "{% if movies contains \"Star Wars\" %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let obj = Object::new();
        context.stack_mut().set_global("movies", Value::Object(obj));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn contains_with_array_and_match() {
        let text = "{% if movies contains \"Star Wars\" %}if true{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let arr = vec![
            Value::scalar("Star Wars"),
            Value::scalar("Star Trek"),
            Value::scalar("Alien"),
        ];
        context.stack_mut().set_global("movies", Value::Array(arr));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");
    }

    #[test]
    fn contains_with_array_and_no_match() {
        let text = "{% if movies contains \"Star Wars\" %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let arr = vec![Value::scalar("Alien")];
        context.stack_mut().set_global("movies", Value::Array(arr));
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn multiple_conditions_and() {
        let text = "{% if 1 == 1 and 2 == 2 %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");

        let text = "{% if 1 == 1 and 2 != 2 %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn multiple_conditions_or() {
        let text = "{% if 1 == 1 or 2 != 2 %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");

        let text = "{% if 1 != 1 or 2 != 2 %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if false");
    }

    #[test]
    fn multiple_conditions_and_or() {
        let text = "{% if 1 == 1 or 2 == 2 and 3 != 3 %}if true{% else %}if false{% endif %}";
        let template = compiler::parse(text, &options())
            .map(interpreter::Template::new)
            .unwrap();

        let mut context = Context::new();
        let output = template.render(&mut context).unwrap();
        assert_eq!(output, "if true");
    }
}
