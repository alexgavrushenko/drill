use std::collections::HashMap;

use colored::*;
use regex::{Captures, Regex};
use serde_json::Value;
use yaml_rust::Yaml;

static INTERPOLATION_PREFIX: &'static str = "{{";
static INTERPOLATION_SUFFIX: &'static str = "}}";

pub struct Interpolator<'a> {
  context: &'a HashMap<String, Yaml>,
  responses: &'a HashMap<String, Value>,
  regexp: Regex
}

impl<'a> Interpolator<'a> {
  pub fn new(context: &'a HashMap<String, Yaml>, responses: &'a HashMap<String, Value>) -> Interpolator<'a> {
    let regexp = format!(
      "{}{}{}",
      INTERPOLATION_PREFIX,
      r" *([a-zA-Z\._]+[a-zA-Z\._0-9]*) *",
      INTERPOLATION_SUFFIX
    );
    Interpolator {
      context: context,
      responses: responses,
      regexp: Regex::new(regexp.as_str()).unwrap()
    }
  }

  pub fn has_interpolations(text: &String) -> bool {
    text.contains(INTERPOLATION_SUFFIX)
  }

  pub fn resolve(&self, url: &String) -> String {
    self.regexp.replace_all(url.as_str(), |caps: &Captures| {
      let capture = &caps[1];

      if let Some(item) = self.resolve_context_interpolation(&capture) {
        return item.to_string();
      }

      if let Some(item) = self.resolve_responses_interpolation(&capture) {
        return item.to_string();
      }

      panic!("{} Unknown '{}' variable!", "WARNING!".yellow().bold(), &capture);
    })
    .to_string()
  }

  // TODO: Refactor this function to support multiple levels
  fn resolve_responses_interpolation(&self, capture: &str) -> Option<String> {
    let cap_path: Vec<&str> = capture.split('.').collect();

    let (cap_root, cap_tail) = cap_path.split_at(1);

    match self.responses.get(cap_root[0]) {
      Some(value) => {
        return Some(value[cap_tail[0]].to_string());
      }
      _ => None,
    }
  }

  // TODO: Refactor this function to support multiple levels
  fn resolve_context_interpolation(&self, capture: &str) -> Option<String> {
    let cap_path: Vec<&str> = capture.split('.').collect();

    let (cap_root, cap_tail) = cap_path.split_at(1);

    match self.context.get(cap_root[0]) {
      Some(value) => {
        if let Some(vs) = value.as_str() {
          return Some(vs.to_string());
        }

        if let Some(vi) = value.as_i64() {
          return Some(vi.to_string());
        }

        if let Some(vh) = value.as_hash() {
          let item_key = Yaml::String(cap_tail[0].to_string());

          match vh.get(&item_key) {
            Some(value) => {
              if let Some(vs) = value.as_str() {
                return Some(vs.to_string());
              }

              if let Some(vi) = value.as_i64() {
                return Some(vi.to_string());
              }

              panic!("{} Unknown type for '{}' variable!", "WARNING!".yellow().bold(), &capture);
            }
            _ => {
              panic!("{} Unknown '{}' variable!", "WARNING!".yellow().bold(), &capture);
            }
          }
        }

        panic!("{} Unknown type for '{}' variable!", "WARNING!".yellow().bold(), &capture);
      }
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json;
  use serde_json::Value;

  #[test]
  fn interpolates_variables() {
    let mut context: HashMap<String, Yaml> = HashMap::new();
    let responses: HashMap<String, Value> = HashMap::new();

    context.insert(String::from("user_Id"), Yaml::String(String::from("12")));

    let interpolator = Interpolator::new(&context, &responses);
    let url = String::from("http://example.com/users/{{ user_Id }}/view/{{ user_Id }}");
    let interpolated = interpolator.resolve(&url);

    assert_eq!(interpolated, "http://example.com/users/12/view/12");
  }

  #[test]
  fn interpolates_responses() {
    let context: HashMap<String, Yaml> = HashMap::new();
    let mut responses: HashMap<String, Value> = HashMap::new();

    let data = String::from("{ \"bar\": 12 }");
    let value: serde_json::Value = serde_json::from_str(&data).unwrap();
    responses.insert(String::from("foo"), value);

    let interpolator = Interpolator::new(&context, &responses);
    let url = String::from("http://example.com/users/{{ foo.bar }}");
    let interpolated = interpolator.resolve(&url);

    assert_eq!(interpolated, "http://example.com/users/12");
  }

  #[test]
  #[should_panic]
  fn interpolates_missing_variable() {
    let context: HashMap<String, Yaml> = HashMap::new();
    let responses: HashMap<String, Value> = HashMap::new();

    let interpolator = Interpolator::new(&context, &responses);
    let url = String::from("/users/{{ userId }}");
    interpolator.resolve(&url);
  }

  #[test]
  fn interpolates_numnamed_variables() {
    let mut context: HashMap<String, Yaml> = HashMap::new();
    let responses: HashMap<String, Value> = HashMap::new();

    context.insert(String::from("zip5"), Yaml::String(String::from("90210")));

    let interpolator = Interpolator::new(&context, &responses);
    let url = String::from("http://example.com/postalcode/{{ zip5 }}/view/{{ zip5 }}");
    let interpolated = interpolator.resolve(&url);

    assert_eq!(interpolated, "http://example.com/postalcode/90210/view/90210");
  }

  #[test]
  fn interpolates_bad_numnamed_variable_names() {
    let mut context: HashMap<String, Yaml> = HashMap::new();
    let responses: HashMap<String, Value> = HashMap::new();

    context.insert(String::from("5digitzip"), Yaml::String(String::from("90210")));

    let interpolator = Interpolator::new(&context, &responses);
    let url = String::from("http://example.com/postalcode/{{ 5digitzip }}/view/{{ 5digitzip }}");
    let interpolated = interpolator.resolve(&url);

    assert_eq!(interpolated, "http://example.com/postalcode/{{ 5digitzip }}/view/{{ 5digitzip }}");
  }
}
