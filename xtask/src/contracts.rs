use crate::{Result, root};
use anagram_core::{contracts::JobStatus, policy::DeploymentLimits, request, solve};
use schemars::schema_for;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

fn ts(s: &Value) -> Result<String> {
    if let Some(b) = s.as_bool() {
        return Ok(if b { "unknown" } else { "never" }.into());
    }
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        return Ok(r
            .strip_prefix("#/$defs/")
            .ok_or("Unsupported schema reference")?
            .into());
    }
    if let Some(v) = s.get("const") {
        return Ok(v.to_string());
    }
    if let Some(v) = s.get("enum").and_then(Value::as_array) {
        return Ok(v
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(" | "));
    }
    for (key, op) in [("anyOf", " | "), ("oneOf", " | "), ("allOf", " & ")] {
        if let Some(v) = s.get(key).and_then(Value::as_array) {
            return Ok(v
                .iter()
                .map(|v| ts(v).map(|v| format!("({v})")))
                .collect::<Result<Vec<_>>>()?
                .join(op));
        }
    }
    if let Some(v) = s.get("type").and_then(Value::as_array) {
        return Ok(v
            .iter()
            .map(|t| {
                let mut s = s.clone();
                s["type"] = t.clone();
                ts(&s)
            })
            .collect::<Result<Vec<_>>>()?
            .join(" | "));
    }
    Ok(match s.get("type").and_then(Value::as_str) {
        Some("string" | "boolean" | "null") => s["type"].as_str().unwrap().into(),
        Some("integer" | "number") => "number".into(),
        Some("array") => format!("Array<{}>", ts(s.get("items").ok_or("Missing items")?)?),
        Some("object") => {
            let mut fields = Vec::new();
            if let Some(props) = s.get("properties").and_then(Value::as_object) {
                for (k, v) in props {
                    let required = s["required"]
                        .as_array()
                        .is_some_and(|a| a.contains(&json!(k)));
                    fields.push(format!(
                        "  {}{}: {};",
                        json!(k),
                        if required { "" } else { "?" },
                        ts(v)?
                    ));
                }
            }
            if let Some(patterns) = s.get("patternProperties").and_then(Value::as_object) {
                if patterns.len() != 1 || !patterns.contains_key(r"^\d+$") {
                    return Err("Unsupported schema key patterns".into());
                }
                fields.push(format!("  [key: string]: {};", ts(&patterns[r"^\d+$"])?));
            }
            let extra = s.get("additionalProperties").unwrap_or(&Value::Bool(true));
            if extra != &Value::Bool(false) {
                fields.push(format!("  [key: string]: {};", ts(extra)?));
            }
            format!("{{\n{}\n}}", fields.join("\n"))
        }
        _ => return Err(format!("Unsupported schema node {s}").into()),
    })
}
pub fn generate(check: bool) -> Result<()> {
    let schemas = json!({"GenerateRequest":schema_for!(request::GenerateRequest),"Generated":schema_for!(request::Generated),"SolveRequest":schema_for!(solve::Request),"SolveResult":schema_for!(solve::Result),"SolverError":schema_for!(request::Error),"ErrorResponse":schema_for!(request::ErrorResponse),"JobStatus":schema_for!(JobStatus),"DeploymentLimits":schema_for!(DeploymentLimits)});
    let dir = root().join("contracts/generated");
    let mut outputs = BTreeMap::new();
    for (name, schema) in schemas.as_object().unwrap() {
        outputs.insert(
            format!("{name}.schema.json"),
            serde_json::to_string_pretty(schema)? + "\n",
        );
        let mut lines =
            vec!["// Generated from Rust by cargo xtask contracts. Do not edit.".into()];
        if let Some(defs) = schema.get("$defs").and_then(Value::as_object) {
            for (k, v) in defs {
                lines.push(format!("export type {k} = {};", ts(v)?));
            }
        }
        lines.push(format!("export type {name} = {};", ts(schema)?));
        outputs.insert(format!("{name}.ts"), lines.join("\n\n") + "\n");
    }
    fs::create_dir_all(&dir)?;
    for entry in fs::read_dir(&dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if (name.ends_with(".schema.json") || name.ends_with(".ts")) && !outputs.contains_key(&name)
        {
            return Err(format!("Stale contract; remove explicitly: {name}").into());
        }
    }
    for (name, content) in &outputs {
        let path = dir.join(name);
        if check {
            if fs::read_to_string(&path)?.replace("\r\n", "\n") != *content {
                return Err(format!("Contract drift: {}", path.display()).into());
            }
        } else {
            fs::write(path, content)?;
        }
    }
    println!(
        "{} {} schema/TypeScript artifacts",
        if check { "Checked" } else { "Generated" },
        outputs.len()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_structural_types() {
        assert_eq!(ts(&json!({"type":"object","properties":{"x":{"type":["integer","null"]}},"required":["x"],"additionalProperties":false})).unwrap(), "{\n  \"x\": number | null;\n}");
        assert!(ts(&json!({"$ref":"remote"})).is_err());
        assert!(ts(&json!({"type":"object","patternProperties":{"other":true}})).is_err());
        assert_eq!(
            ts(&json!({"oneOf":[{"const":"a"},false]})).unwrap(),
            "(\"a\") | (never)"
        );
    }
}
