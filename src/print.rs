use crate::export_info::{self, Feature};
use colored::{Color, Colorize};

const LIGHT_RED: Color = Color::TrueColor {
    r: 153,
    g: 53,
    b: 53,
};

const LIGHT_GREY: Color = Color::TrueColor {
    r: 120,
    g: 120,
    b: 120,
};

pub fn pretty_print(mut export_info: export_info::ExportInfo) {
    let globally_active = export_info.root_package.globally_active;

    pretty_print_package(export_info.root_package);

    if !globally_active {
        return;
    }

    export_info.dependencies.sort();
    export_info.dependencies.into_iter().for_each(|dependency| {
        pretty_print_package(dependency);
    });
}

pub fn pretty_print_package(package: export_info::Package) {
    let crate_color = if package.globally_active {
        Color::Green
    } else {
        LIGHT_RED
    };

    // Print version (if active)
    print!(
        "{}",
        format!("`{}`", package.name.bold())
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

    // Print optional and/or active
    let title_optional = match (package.optional, package.globally_active) {
        (true, true) => "(optional)".color(Color::Cyan),
        (true, false) => "(optional)".color(crate_color),
        (false, true) => "".normal(),
        (false, false) => "".normal(),
    };
    print!(" {}", title_optional);
    if !package.active {
        print!("{}", " not active".color(LIGHT_RED))
    }
    if package.optional && !package.active && package.globally_active {
        print!(
            "{}",
            ", but activated somewhere else in the tree".color(LIGHT_GREY)
        )
    }
    println!();
    if !package.active && !package.globally_active {
        return;
    }

    // Print "No features"
    if package.features.is_empty() {
        println!("  {}", "[no features]".color(LIGHT_GREY).italic());
        return;
    }

    pretty_print_features(package.features);
}

fn pretty_print_features(mut features: Vec<Feature>) {
    features.sort();
    features.iter().for_each(|feature| {
        let icon = if feature.active {
            "* ".green()
        } else {
            "- ".bright_red()
        };
        let name_colored = if feature.name.eq("default") {
            feature.name.yellow()
        } else {
            feature.name.normal()
        };
        print!("  {}{} = [", icon, name_colored);
        let mut childs = feature.childs.to_owned();
        childs.sort();
        childs.iter().enumerate().for_each(|(i, child)| {
            let child_str = format!("\"{}\"", child);
            let child_colored = if child.starts_with("dep:") {
                child_str.cyan()
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
