use super::*;

pub(super) fn check(root: &Path, gui: &mut Gui) -> anyhow::Result<()> {
    let project = root.join("project");
    let live = root.join("live");
    let launches = root.join("launches");
    fs::create_dir(&project)?;
    fs::create_dir(&live)?;
    fs::write(
        root.join("shell"),
        format!(
            "#!/bin/sh\npwd -L >> '{}'\ncd '{}' || exit 1\nprintf '\\033]7;file://localhost%s\\007' \"$PWD\"\nexec /bin/sh\n",
            launches.display(),
            live.display()
        ),
    )?;
    let id = command(&["create", "--cwd", project.to_str().unwrap()])?
        .trim()
        .to_owned();
    wait_for(|| {
        Ok(sessions()?
            .iter()
            .any(|s| s.id == id && Path::new(&s.cwd) == live))
    })?;
    for count in [2, 3] {
        if count == 3 {
            step(gui, Step::InSession)?;
        }
        step(gui, Step::NewTab)?;
        wait_for(|| {
            Ok(sessions()?
                .iter()
                .any(|s| s.id == id && s.tab_count == count && Path::new(&s.cwd) == live))
        })?;
    }
    step(gui, Step::SelectFirstTab)?;
    std::thread::sleep(Duration::from_millis(150));
    step(gui, Step::CloseFirstTab)?;
    wait_for(|| Ok(sessions()?.iter().any(|s| s.id == id && s.tab_count == 2)))?;
    step(gui, Step::NewTab)?;
    wait_for(|| Ok(fs::read_to_string(&launches)?.lines().count() == 4))?;
    let starts = fs::read_to_string(&launches)?;
    ensure!(
        starts
            .lines()
            .all(|p| Path::new(p).canonicalize().ok() == project.canonicalize().ok()),
        "new tabs did not retain session cwd: {starts}"
    );

    let inherited = command(&["create"])?.trim().to_owned();
    wait_for(|| Ok(fs::read_to_string(&launches)?.lines().count() == 5))?;
    ensure!(
        Path::new(fs::read_to_string(&launches)?.lines().last().unwrap()).canonicalize()?
            == live.canonicalize()?,
        "new session did not follow live cwd"
    );
    command(&["close", &inherited])?;
    command(&["focus", &id])?;

    for failure in ["missing", "file", "denied"] {
        match failure {
            "missing" => fs::remove_dir(&project)?,
            "file" => fs::write(&project, "not a directory")?,
            "denied" => {
                fs::remove_file(&project)?;
                fs::create_dir(&project)?;
                fs::set_permissions(&project, fs::Permissions::from_mode(0o0))?;
            }
            _ => unreachable!(),
        }
        let before = sessions()?;
        step(gui, Step::NewTab)?;
        std::thread::sleep(Duration::from_millis(250));
        let after = sessions()?;
        ensure!(
            serde_json::to_value(before)? == serde_json::to_value(after)?,
            "{failure} cwd changed live sessions"
        );
        ensure!(
            fs::read_to_string(&launches)?.lines().count() == 5,
            "{failure} cwd launched a shell"
        );
    }
    fs::set_permissions(&project, fs::Permissions::from_mode(0o700))?;
    command(&["close", &id, "--force"])?;
    fs::write(root.join("shell"), "#!/bin/sh\nexec /bin/sh\n")?;
    println!(
        "cli_live: retained cwd, cd reports, original-tab closure, settings, and failed launches passed"
    );
    Ok(())
}
