/*
 * Copyright (c) 2025 Bastiaan van der Plaat
 *
 * SPDX-License-Identifier: MIT
 */

use std::collections::HashMap;
use std::fs;
use std::process::{Command, exit};

use crate::Bobje;
use crate::args::Profile;
use crate::executor::Executor;

// MARK: Javac tasks
pub(crate) fn detect_java(bobje: &Bobje) -> bool {
    bobje
        .source_files
        .iter()
        .any(|path| path.ends_with(".java"))
}

pub(crate) fn generate_javac_tasks(bobje: &Bobje, executor: &mut Executor) {
    let classes_dir = format!("{}/{}/classes", bobje.target_dir, bobje.profile);
    let modules = find_modules(bobje);
    let module_deps = find_dependencies(&modules);

    let mut javac_flags = format!(
        "-Xlint -Werror {}",
        if bobje.profile == Profile::Release {
            "-g:none"
        } else {
            "-g"
        }
    );
    if !bobje.manifest.build.javac_flags.is_empty() {
        javac_flags.push(' ');
        javac_flags.push_str(&bobje.manifest.build.javac_flags);
    }

    let classpath = format!(
        "{}{}",
        classes_dir,
        if !bobje.manifest.build.classpath.is_empty() {
            format!(":{}", bobje.manifest.build.classpath.join(":"))
        } else {
            "".to_string()
        }
    );

    for module in &modules {
        let mut inputs = module.source_files.clone();
        if let Some(dependencies) = module_deps.get(&module.name) {
            for dependency_module in dependencies {
                let classes_module_dir = format!(
                    "{}/{}",
                    classes_dir,
                    dependency_module.name.replace('.', "/")
                );
                if !inputs.contains(&classes_module_dir) {
                    inputs.push(classes_module_dir);
                }
            }
        }
        executor.add_task_cmd(
            format!(
                "javac {} -cp {} -d {} {}",
                javac_flags,
                classpath,
                classes_dir,
                module.source_files.join(" ")
            ),
            inputs,
            vec![format!("{}/{}", classes_dir, module.name.replace('.', "/"))],
        );
    }
}

pub(crate) fn run_java_class(bobje: &Bobje) {
    let status = Command::new("java")
        .arg("-cp")
        .arg(format!("{}/{}/classes", bobje.target_dir, bobje.profile))
        .arg(find_main_class(bobje).unwrap_or_else(|| {
            eprintln!("Can't find main class");
            exit(1);
        }))
        .status()
        .expect("Failed to execute java");
    exit(status.code().unwrap_or(1));
}

// MARK: Jar tasks
pub(crate) fn detect_jar(bobje: &Bobje) -> bool {
    bobje.manifest.package.metadata.jar.is_some()
}

pub(crate) fn generate_jar_tasks(bobje: &Bobje, executor: &mut Executor) {
    let classes_dir = format!("{}/{}/classes", bobje.target_dir, bobje.profile);
    let modules = find_modules(bobje);

    let main_class = bobje
        .manifest
        .package
        .metadata
        .jar
        .as_ref()
        .and_then(|jar| jar.main_class.clone())
        .unwrap_or_else(|| {
            find_main_class(bobje).unwrap_or_else(|| {
                eprintln!("Can't find main class");
                exit(1);
            })
        });

    let jar_file = format!(
        "{}/{}-{}.jar",
        bobje.out_dir(),
        bobje.manifest.package.name,
        bobje.manifest.package.version
    );
    executor.add_task_cmd(
        format!("jar cfe {} {} -C {} .", jar_file, main_class, classes_dir),
        modules
            .iter()
            .map(|module| format!("{}/{}", classes_dir, module.name.replace('.', "/")))
            .collect::<Vec<_>>(),
        vec![jar_file],
    );
}

pub(crate) fn run_jar(bobje: &Bobje) {
    let status = Command::new("java")
        .arg("-jar")
        .arg(format!(
            "{}/{}-{}.jar",
            bobje.out_dir(),
            bobje.manifest.package.name,
            bobje.manifest.package.version
        ))
        .status()
        .expect("Failed to execute java");
    exit(status.code().unwrap_or(1));
}

// MARK: Utils
fn get_class_name(source_file: &str) -> String {
    source_file
        .split("src/")
        .nth(1)
        .or_else(|| source_file.split("src-gen/").nth(1))
        .expect("Should be some")
        .trim_end_matches(".java")
        .replace(['/', '\\'], ".")
}

fn get_module_name(source_file: &str) -> String {
    get_class_name(source_file)
        .rsplit_once('.')
        .map_or("", |(prefix, _)| prefix)
        .to_string()
}

#[derive(Clone)]
pub(crate) struct Module {
    pub name: String,
    pub source_files: Vec<String>,
}

pub(crate) fn find_modules(bobje: &Bobje) -> Vec<Module> {
    let mut modules = Vec::new();
    for dependency_bobje in bobje.dependencies.values() {
        for module in find_modules(dependency_bobje) {
            modules.push(module);
        }
    }
    for source_file in &bobje.source_files {
        if source_file.ends_with(".java") {
            let module_name = get_module_name(source_file);
            if let Some(module) = modules.iter_mut().find(|m| m.name == module_name) {
                module.source_files.push(source_file.to_string());
            } else {
                modules.push(Module {
                    name: module_name.clone(),
                    source_files: vec![source_file.to_string()],
                });
            }
        }
    }
    modules
}

fn find_dependencies(modules: &Vec<Module>) -> HashMap<String, Vec<Module>> {
    let mut module_deps = HashMap::new();
    for module in modules {
        for source_file in &module.source_files {
            if let Ok(contents) = fs::read_to_string(source_file) {
                for other_module in modules {
                    if other_module.name == module.name {
                        continue;
                    }
                    let re = regex::Regex::new(&format!(r"import {}.\w+;", other_module.name))
                        .expect("Failed to compile regex");
                    if re.is_match(&contents) {
                        module_deps
                            .entry(module.name.clone())
                            .or_insert_with(Vec::new)
                            .push(other_module.clone());
                    }
                }
            }
        }
    }
    module_deps
}

fn find_main_class(bobje: &Bobje) -> Option<String> {
    let re = regex::Regex::new(r"(public\s+)?static\s+void\s+main\s*\(")
        .expect("Failed to compile regex");
    for source_file in &bobje.source_files {
        if let Ok(contents) = fs::read_to_string(source_file) {
            if re.is_match(&contents) {
                return Some(get_class_name(source_file));
            }
        }
    }
    None
}
