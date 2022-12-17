use crate::export_info;
use colored::{Color, Colorize};

pub fn pretty_print(mut export_info: export_info::ExportInfo) {
    let active = export_info.root_package.active;

    pretty_print_package(export_info.root_package);

    if !active {
        return;
    }

    export_info.dependencies.sort();
    export_info.dependencies.into_iter().for_each(|dependency| {
        pretty_print_package(dependency);
    });
}

pub fn pretty_print_package(mut package: export_info::Package) {
    let crate_color = if package.active {
        Color::Cyan
    } else {
        Color::TrueColor {
            r: 153,
            g: 53,
            b: 53,
        }
    };
    let title_optional = match (package.optional, package.active) {
        (true, true) => "(optional)".color(Color::Yellow),
        (true, false) => "(optional)".color(crate_color),
        (false, true) => "".normal(),
        (false, false) => "".normal(),
    };

    print!(
        "{}",
        // FIXME: if optional and not active, the version should be considered as unknown
        format!("`{}`", package.name.bold(),)
            .color(crate_color)
            .underline(),
    );
    if package.active {
        print!(
            "{}{}",
            " ".green().underline(),
            package.version.color(crate_color).underline()
        );
    }
    print!(" {}", title_optional);
    if !package.active {
        print!("{}", " not active".color(crate_color))
    }
    if package.optional && !package.active && package.globally_active {
        print!("{}", " but globally activated".color(Color::Cyan))
    }
    println!();
    if !package.active && !package.globally_active {
        return;
    }

    if package.features.is_empty() {
        println!("  {}", "[no features]".bright_black().italic());
        return;
    }

    package.features.sort();
    package.features.iter().for_each(|feature| {
        let icon = if feature.active {
            "* ".green()
        } else {
            "- ".bright_red()
        };
        print!("  {}{} = [", icon, feature.name.green());
        let mut childs = feature.childs.to_owned();
        childs.sort();
        childs.iter().enumerate().for_each(|(i, child)| {
            let child_str = format!("\"{}\"", child);
            let child_colored = if child.starts_with("dep:") {
                child_str.yellow()
            } else {
                child_str.normal()
            };
            print!("{child_colored}");
            if i + 1 != childs.len() {
                print!(", ");
            }
        });
        println!("]");
    });
}
