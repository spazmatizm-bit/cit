use colored::*;
use std::fs;
use std::io::Write;
use crate::repo::{find_all_packages, Package};

pub fn source_install(pkgname: &str, repo_name: Option<&str>) {
    println!("\n{}", format!("Source installing {}...", pkgname).bold().yellow());
    
    // 1. Находим пакет
    let packages = find_all_packages(pkgname);
    if packages.is_empty() {
        println!("{}", format!("Package '{}' not found", pkgname).red());
        return;
    }
    
    let pkg = if let Some(repo) = repo_name {
        packages.iter().find(|p| p.repo == repo)
    } else {
        packages.first()
    };
    
    let pkg = match pkg {
        Some(p) => p,
        None => {
            println!("{}", "Package not found in specified repository".red());
            return;
        }
    };
    
    println!("{}: {}", "Source installing".green().bold(), pkg.name.cyan());
    println!("{}: {}", "Repository".bold(), pkg.repo.yellow());
    println!("{}: {}", "Version".bold(), pkg.version.green());
    
    // 2. Определяем тип сборки
    let build_type = if pkg.repo.contains("gentoo") || pkg.repo.contains("portage") {
        "Gentoo (ebuild)"
    } else if pkg.repo.contains("arch") {
        "Arch (PKGBUILD)"
    } else if pkg.repo.contains("deb") || pkg.repo.contains("devuan") {
        "Debian (debhelper)"
    } else {
        "Generic (configure/make)"
    };
    
    println!("{}: {}", "Build type".bold(), build_type);
    
    // 3. Запрашиваем подтверждение
    print!("\n{}", "Proceed with source build? [Y/n]: ".bold());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    if input.trim().to_lowercase() != "y" && !input.trim().is_empty() {
        println!("{}", "Source build cancelled.".yellow());
        return;
    }
    
    // 4. Запускаем сборку
    match build_type {
        "Gentoo (ebuild)" => build_gentoo_pkg(pkg),
        "Arch (PKGBUILD)" => build_arch_pkg(pkg),
        "Debian (debhelper)" => build_debian_pkg(pkg),
        _ => build_generic_pkg(pkg),
    }
}

fn build_arch_pkg(pkg: &Package) {
    println!("\n{}", "Building Arch package from source...".bold());
    
    let build_dir = format!("/tmp/cit-build/{}", pkg.name);
    if let Err(e) = fs::create_dir_all(&build_dir) {
        println!("  ✗ Failed to create build dir: {}", e);
        return;
    }
    
    println!("  → Build directory: {}", build_dir);
    println!("  ℹ Arch source build not fully implemented yet");
    println!("  → Use: git clone ... && cd ... && makepkg -si");
}

fn build_debian_pkg(pkg: &Package) {
    println!("\n{}", "Building Debian package from source...".bold());
    
    let build_dir = format!("/tmp/cit-build/{}", pkg.name);
    if let Err(e) = fs::create_dir_all(&build_dir) {
        println!("  ✗ Failed to create build dir: {}", e);
        return;
    }
    
    println!("  → Build directory: {}", build_dir);
    println!("  ℹ Debian source build not fully implemented yet");
    println!("  → Use: apt-get source {} && cd ... && dpkg-buildpackage", pkg.name);
}

fn build_gentoo_pkg(pkg: &Package) {
    println!("\n{}", "Building Gentoo package from source...".bold());
    
    let build_dir = format!("/tmp/cit-build/{}", pkg.name);
    if let Err(e) = fs::create_dir_all(&build_dir) {
        println!("  ✗ Failed to create build dir: {}", e);
        return;
    }
    
    println!("  → Build directory: {}", build_dir);
    println!("  ℹ Gentoo source build not fully implemented yet");
    println!("  → Use: emerge --buildpkg {} && emerge {}", pkg.name, pkg.name);
}

fn build_generic_pkg(pkg: &Package) {
    println!("\n{}", "Building from generic source...".bold());
    println!("  → No build system detected for {}", pkg.repo);
    println!("  → Try: git clone <url> && ./configure && make && make install");
}
