use crate::export_info::{self, Feature, Optional};
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
    if package.features.is_empty() && package.optionals.is_empty() {
        println!("  {}", "[no features]".color(LIGHT_GREY).italic());
        return;
    }

    pretty_print_features(package.features);
    pretty_print_optionals(package.optionals);
}

fn pretty_print_features(mut features: Vec<Feature>) {
    features.sort();
    features
        .iter()
        // HACK .filter(|f| !f.optional)
        .for_each(|feature| {
            if feature.optional {
                // HACK
                print!("{}", "  ? ".on_magenta());
            } else if feature.active {
                print!("{}", "  * ".green());
            } else {
                print!("{}", "  - ".bright_red());
            }
            if feature.name.eq("default") {
                print!("{}", feature.name.yellow());
            } else {
                print!("{}", feature.name);
            };

            let mut childs = feature.childs.clone();
            childs.sort();
            print!(" = [");
            childs.iter().enumerate().for_each(|(i, child)| {
                let child_str = format!("\"{}\"", child.name);
                let child_colored = if child.optional {
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

fn pretty_print_optionals(mut optionals: Vec<Optional>) {
    optionals.sort();
    optionals.iter().for_each(|optional| {
        if optional.active {
            print!("  {} ", "*".green());
        } else {
            print!("  {} ", "-".bright_red());
        }
        println!(
            "{} {}",
            &optional.name.cyan(),
            "(optional)".color(LIGHT_GREY).italic()
        );
    });
}
