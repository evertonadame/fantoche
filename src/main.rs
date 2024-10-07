use find_config_file::find_path_in_hierarchy;
use fs_extra::file::{copy, CopyOptions};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::{
    fs::{self},
    io,
    path::{Path, PathBuf},
    sync::{mpsc::channel, Arc, Mutex},
};
mod find_config_file;

#[derive(Debug)]
struct ProjectContext {
    name: String,
}

fn main() {
    let processed_paths: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

    let config = find_config_file::get_config_file();

    let (tx, rx) = channel::<(notify::Event, String)>();

    let tx = Arc::new(Mutex::new(tx));

    let mut watchers = vec![];

    for project in &config.projects {
        let exports_path = find_path_in_hierarchy(&project.path)
            .expect("Erro ao encontrar diretório de exportação");

        let exports_path_buf = Path::new(&exports_path).join(&project.exports);
        let exports_path = exports_path_buf.as_path();

        let project_context = ProjectContext {
            name: project.name.clone(),
        };

        let tx_clone = Arc::clone(&tx);

        let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let project_name = project_context.name.clone();
                match res {
                    Ok(event) => match event.kind {
                        EventKind::Create(_)
                        | EventKind::Modify(_)
                        | EventKind::Remove(_)
                        | EventKind::Access(_) => {
                            let tx = tx_clone.lock().unwrap();
                            tx.send((event, project_name))
                                .expect("Erro ao enviar evento");
                        }
                        _ => (),
                    },
                    Err(e) => println!("Erro ao receber evento: {:?}", e),
                }
            },
            Config::default(),
        )
        .expect("Erro ao criar watcher");

        watcher
            .watch(exports_path, RecursiveMode::Recursive)
            .expect("Erro ao observar diretório");

        watchers.push(watcher);
    }

    loop {
        if let Ok((event, project_name)) = rx.recv() {
            process_event(&event, &project_name, &processed_paths);
        }
    }
}

fn process_event(
    event: &notify::Event,
    project_name: &str,
    processed_paths: &Arc<Mutex<HashSet<PathBuf>>>,
) {
    if processed_paths.lock().unwrap().contains(&event.paths[0]) {
        return;
    }

    let config = find_config_file::get_config_file();

    let projects_with_this_project_dependency = config.projects.iter().filter(|project| {
        if let Some(dependencies) = &project.dependencies {
            dependencies
                .iter()
                .any(|dependency| dependency.name == project_name)
        } else {
            false
        }
    });

    for project in projects_with_this_project_dependency {
        let project_path = find_path_in_hierarchy(&project.path)
            .expect("Erro ao encontrar diretório de exportação");

        let dependencies_store = project.dependencies_store.as_ref().unwrap();

        let dependencies_store_path_buf = Path::new(&project_path).join(dependencies_store);

        let dependencies_store_path = dependencies_store_path_buf.as_path();

        let dependency_dist_path = Path::new(&dependencies_store_path);

        let new_dist_path_buf = event.paths[0].clone();
        let new_dist_path = new_dist_path_buf.parent().unwrap();

        let changed_project_with_exports = find_dist_directory(&new_dist_path, &project.exports)
            .expect("Erro ao encontrar diretório dist");

        println!("dependency_dist_path: {:?}", dependency_dist_path);

        replace_dist_contents(
            &event.paths[0].clone(),
            &changed_project_with_exports,
            &dependency_dist_path,
            &project_name,
        )
        .expect("Erro ao substituir dist");
    }
}

fn replace_dist_contents(
    changed: &Path,
    changed_project_with_exports: &Path,
    dependency_dist: &Path,
    project_name: &str,
) -> io::Result<()> {
    let changed = changed.canonicalize().unwrap();
    let path_to_replace_changed = changed.strip_prefix(changed_project_with_exports).unwrap();
    let path_to_replace = dependency_dist
        .join(&project_name)
        .join(path_to_replace_changed);

    if let Some(parent_dir) = path_to_replace.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let mut options = CopyOptions::new();

    options.overwrite = true;

    copy(&changed, &path_to_replace, &options)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    println!("Substituído {:?} por {:?}", path_to_replace, &changed);

    Ok(())
}

fn find_dist_directory(event_path: &Path, dependencies_store: &str) -> Option<PathBuf> {
    // Itera sobre os diretórios pais até encontrar "dist"
    for ancestor in event_path.ancestors() {
        if ancestor.ends_with(&dependencies_store) {
            return Some(ancestor.to_path_buf());
        }
    }
    None // Se não encontrar "dist", retorna None
}
