extern crate chrono;
extern crate liquid;
extern crate liquid_core;
extern crate regex;

pub use liquid_core::Value::Nil;

#[allow(dead_code)]
pub fn date(y: i32, m: u32, d: u32) -> liquid_core::Value {
    use liquid_core::value::{Date, Value};
    Value::scalar(Date::from_ymd(y, m, d))
}

#[allow(dead_code)]
pub fn with_time(_time: &str) -> liquid_core::Value {
    Nil
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! v {
    ($($value:tt)+) => {
        ::liquid_core::value::value!($($value)+)
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! o {
    ($($value:tt)+) => {
        ::liquid_core::object!($($value)+)
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! a {
    ($($value:tt)+) => {
        ::liquid_core::value::array!($($value)+)
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_template_result {
    ($expected:expr, $template:expr, ) => {
        assert_template_result!($expected, $template);
    };
    ($expected:expr, $template:expr) => {
        let assigns = ::liquid_core::Object::default();
        assert_template_result!($expected, $template, assigns);
    };
    ($expected:expr, $template:expr, $assigns: expr, ) => {
        assert_template_result!($expected, $template, $assigns);
    };
    ($expected:expr, $template:expr, $assigns: expr) => {
        let liquid: ::liquid::ParserBuilder = ::liquid::ParserBuilder::with_liquid();
        let liquid = liquid.build().unwrap();
        assert_template_result!($expected, $template, $assigns, liquid);
    };
    ($expected:expr, $template:expr, $assigns: expr, $liquid: expr, ) => {
        assert_template_result!($expected, $template, $assigns, $liquid);
    };
    ($expected:expr, $template:expr, $assigns: expr, $liquid: expr) => {
        let template = $liquid.parse($template.as_ref()).unwrap();
        let rendered = template.render(&$assigns).unwrap();
        assert_eq!($expected, rendered);
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_template_matches {
    ($expected:expr, $template:expr, ) => {
        assert_template_matches!($expected, $template);
    };
    ($expected:expr, $template:expr) => {
        let assigns = liquid::value::Value::default();
        assert_template_matches!($expected, $template, assigns);
    };
    ($expected:expr, $template:expr, $assigns: expr, ) => {
        assert_template_matches!($expected, $template, $assigns);
    };
    ($expected:expr, $template:expr, $assigns: expr) => {
        let template = ::liquid::ParserBuilder::with_liquid()
            .build()
            .unwrap()
            .parse($template.as_ref())
            .unwrap();
        let rendered = template.render(&$assigns).unwrap();

        let expected = $expected;
        println!("pattern={}", expected);
        let expected = regex::Regex::new(expected).unwrap();
        println!("rendered={}", rendered);
        assert!(expected.is_match(&rendered));
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_parse_error {
    ($template:expr, ) => {
        assert_parse_error!($template)
    };
    ($template:expr) => {{
        let liquid = ::liquid::ParserBuilder::with_liquid().build().unwrap();
        assert_parse_error!($template, liquid)
    }};
    ($template:expr, $liquid:expr, ) => {{
        assert_parse_error!($template, $liquid)
    }};
    ($template:expr, $liquid:expr) => {{
        let template = $liquid.parse($template);
        assert!(template.is_err());
        template.err().unwrap()
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_render_error {
    ($template:expr, ) => {
        assert_render_error!($template);
    };
    ($template:expr) => {
        let assigns = ::liquid::Object::default();
        assert_render_error!($template, assigns);
    };
    ($template:expr, $assigns: expr, ) => {
        assert_render_error!($template, $assigns);
    };
    ($template:expr, $assigns: expr) => {
        let template = ::liquid::ParserBuilder::with_liquid()
            .build()
            .unwrap()
            .parse($template.as_ref())
            .unwrap();
        template.render(&$assigns).unwrap_err();
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! filters {
    ($a:ident, $b:expr) => {{
        filters!($a, $b, )
    }};
    ($a:ident, $b:expr, $($c:expr),*) => {{
        let positional = Box::new(vec![$(::liquid_core::Expression::Literal($c)),*].into_iter());
        let keyword = Box::new(Vec::new().into_iter());
        let args = ::liquid_core::compiler::FilterArguments { positional, keyword };

        let context = ::liquid_core::Context::default();

        let filter = ::liquid_core::ParseFilter::parse(&::liquid::filters::std::$a, args).unwrap();
        ::liquid_core::Filter::evaluate(&*filter, &$b, &context).unwrap()
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! filters_fail {
    ($a:ident, $b:expr) => {{
        filters_fail!($a, $b, )
    }};
    ($a:ident, $b:expr, $($c:expr),*) => {{
        let positional = Box::new(vec![$(::liquid_core::Expression::Literal($c)),*].into_iter());
        let keyword = Box::new(Vec::new().into_iter());
        let args = ::liquid_core::compiler::FilterArguments { positional, keyword };

        let context = ::liquid_core::Context::default();

        ::liquid_core::ParseFilter::parse(&::liquid::filters::std::$a, args)
            .and_then(|filter| ::liquid_core::Filter::evaluate(&*filter, &$b, &context))
            .unwrap_err()
    }};
}

#[cfg(feature = "jekyll-filters")]
#[allow(unused_macros)]
#[macro_export]
macro_rules! jekyll_filters {
    ($a:ident, $b:expr) => {{
        jekyll_filters!($a, $b, )
    }};
    ($a:ident, $b:expr, $($c:expr),*) => {{
        let positional = Box::new(vec![$(::liquid_core::Expression::Literal($c)),*].into_iter());
        let keyword = Box::new(Vec::new().into_iter());
        let args = ::liquid_core::compiler::FilterArguments { positional, keyword };

        let context = ::liquid_core::Context::default();

        let filter = ::liquid_core::ParseFilter::parse(&::liquid::filters::jekyll::$a, args).unwrap();
        ::liquid_core::Filter::evaluate(&*filter, &$b, &context).unwrap()
    }};
}

#[cfg(feature = "jekyll-filters")]
#[allow(unused_macros)]
#[macro_export]
macro_rules! jekyll_filters_fail {
    ($a:ident, $b:expr) => {{
        jekyll_filters_fail!($a, $b, )
    }};
    ($a:ident, $b:expr, $($c:expr),*) => {{
        let positional = Box::new(vec![$(::liquid_core::Expression::Literal($c)),*].into_iter());
        let keyword = Box::new(Vec::new().into_iter());
        let args = ::liquid_core::compiler::FilterArguments { positional, keyword };

        let context = ::liquid_core::Context::default();

        ::liquid_core::ParseFilter::parse(&::liquid::filters::jekyll::$a, args)
            .and_then(|filter| ::liquid_core::Filter::evaluate(&*filter, &$b, &context))
            .unwrap_err()
    }};
}
