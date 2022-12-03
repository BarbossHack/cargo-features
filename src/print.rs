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
    let title_optional = match package.optional {
        true => "(optional)",
        false => "",
    };
    println!(
        "{}",
        format!(
            "`{}` {} {}",
            package.name.bold(),
            package.version,
            title_optional
        )
        .color(Color::Green)
        .underline(),
    );

    package.features.sort();
    package.features.iter().for_each(|feature| {
        let icon = if feature.active {
            "* ".green()
        } else {
            "- ".bright_red()
        };
        println!("{}{} = {:?} ", icon, feature.name, feature.childs);
    });
}
