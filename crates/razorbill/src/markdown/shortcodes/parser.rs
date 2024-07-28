use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use serde_json::{to_value, Map, Value};

use crate::markdown::shortcodes::SHORTCODE_PLACEHOLDER;
use crate::markdown::ShortcodeCall;

#[derive(Parser)]
#[grammar = "markdown/shortcodes/grammar.pest"]
pub struct ShortcodeParser;

pub fn parse_document(
    document: &str,
) -> Result<(String, Vec<ShortcodeCall>), pest::error::Error<Rule>> {
    let mut pairs = match ShortcodeParser::parse(Rule::document, document) {
        Ok(pairs) => pairs,
        Err(err) => Err(err)?,
    };

    let mut shortcode_calls = Vec::new();
    let mut output = String::with_capacity(document.len());

    for pair in pairs.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::text => output.push_str(pair.as_span().as_str()),
            Rule::shortcode_call => {
                let start = output.len();
                let end = start + SHORTCODE_PLACEHOLDER.len();
                let (name, args) = parse_shortcode_call(pair);
                shortcode_calls.push(ShortcodeCall {
                    name,
                    args,
                    span: start..end,
                });
                output.push_str(SHORTCODE_PLACEHOLDER);
            }
            Rule::EOI => (),
            _ => unreachable!(),
        }
    }

    Ok((output, shortcode_calls))
}

fn parse_shortcode_call(pair: Pair<Rule>) -> (String, Map<String, Value>) {
    let mut name = None;
    let mut args = Map::new();

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::ident => {
                name = Some(parse_ident(pair));
            }
            Rule::arg => {
                let mut arg_name = None;
                let mut arg_value = None;

                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::ident => {
                            arg_name = Some(parse_ident(pair));
                        }
                        Rule::literal => {
                            arg_value = Some(parse_literal(pair));
                        }
                        _ => unreachable!("Failed to parse arg: {pair:?}"),
                    }
                }

                args.insert(arg_name.unwrap(), arg_value.unwrap());
            }
            _ => unreachable!("Failed to parse shortcode call: {pair:?}"),
        }
    }

    (name.unwrap(), args)
}

fn parse_literal(pair: Pair<Rule>) -> Value {
    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::boolean => {
                let value = match pair.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => unreachable!("Failed to parse boolean literal"),
                };
                return Value::Bool(value);
            }
            Rule::string => return Value::String(unquote_string(pair.as_str())),
            Rule::float => return to_value(pair.as_str().parse::<f64>().unwrap()).unwrap(),
            Rule::int => return to_value(pair.as_str().parse::<i64>().unwrap()).unwrap(),
            Rule::array => {
                let mut values = Vec::new();
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::literal => values.push(parse_literal(pair)),
                        _ => unreachable!("Failed to parse array of literals: {pair:?}"),
                    }
                }

                return Value::Array(values);
            }
            pair => unreachable!("Unexpected literal: {pair:?}"),
        }
    }

    panic!("Failed to parse literal")
}

fn unquote_string(value: &str) -> String {
    match value.chars().next().unwrap() {
        '"' => value.replace('"', ""),
        '\'' => value.replace('\'', ""),
        '`' => value.replace('`', ""),
        _ => unreachable!("Failed to unquote string: {value:?}"),
    }
}

fn parse_ident(pair: Pair<Rule>) -> String {
    pair.as_span().as_str().to_string()
}
