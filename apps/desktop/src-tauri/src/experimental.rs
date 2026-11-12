//! Explicit, bounded local training, sharing the desktop's single worker owner.
use crate::jobs::Desktop;
use anagram_core::{control::Control, training};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};

#[derive(Clone, Default, Serialize)]
pub struct Snapshot {
    pub active: bool,
    pub report: Option<Value>,
    pub error: Option<String>,
}
#[derive(Deserialize)]
struct Dataset {
    groups: Vec<training::Group>,
}

impl Desktop {
    #[cfg(test)]
    pub fn train_model(&self, dataset: String, epochs: usize, folds: usize) -> Result<(), String> {
        self.train_model_options(
            dataset,
            training::Options {
                epochs,
                folds,
                ..Default::default()
            },
        )
    }
    pub fn train_model_options(
        &self,
        dataset: String,
        options: training::Options,
    ) -> Result<(), String> {
        let (epochs, folds) = (options.epochs, options.folds);
        if !(1..=200).contains(&epochs) || !(2..=10).contains(&folds) {
            return Err("Experimental training allows 1–200 epochs and 2–10 folds".into());
        }
        if !options.learning_rate.is_finite()
            || !(0.000001..=1.0).contains(&options.learning_rate)
            || !options.l2.is_finite()
            || !(0.0..=1.0).contains(&options.l2)
        {
            return Err(
                "Experimental training requires learning rate 0.000001–1 and L2 0–1".into(),
            );
        }
        let directory = self.data.join("models");
        self.operation(60, move |control| {
            train(Path::new(&dataset), &directory, &options, control)
        })
    }
}

fn train(
    dataset: &Path,
    directory: &Path,
    options: &training::Options,
    control: &Control,
) -> Result<Value, String> {
    let fail = |e: std::io::Error| e.to_string();
    let mut bytes = Vec::new();
    control
        .reader(fs::File::open(dataset).map_err(fail)?)
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(fail)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Training dataset exceeds 4 MiB".into());
    }
    let dataset: Dataset = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Expected prepared ranker-build JSON with groups: {e}"))?;
    if dataset.groups.len() > 1000 {
        return Err("Experimental training allows at most 1,000 groups".into());
    }
    let report = training::train(&dataset.groups, options, control).map_err(|e| e.message)?;
    control.check().map_err(str::to_owned)?;
    fs::create_dir_all(directory).map_err(fail)?;
    let mut file = tempfile::Builder::new()
        .prefix("ranker-")
        .suffix(".json")
        .tempfile_in(directory)
        .map_err(fail)?;
    serde_json::to_writer_pretty(&mut file, &report.model).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(fail)?;
    control.check().map_err(str::to_owned)?;
    let (_, path) = file.keep().map_err(|e| e.to_string())?;
    let result = json!({"baseline":report.baseline,"held_out":report.held_out,"model_path":path,"report_path":path.with_extension("report.json"),"options":options});
    fs::write(
        path.with_extension("report.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(fail)?;
    Ok(result)
}
