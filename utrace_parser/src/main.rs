use object::{Object, ObjectSection, ObjectSymbol};
use std::{borrow, fs};

#[derive(Debug)]
struct TraceData {
    id: u64,
    name: String,
    path: Option<String>,
    file_name: Option<String>,
    line: Option<u64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "../hello_world/target/thumbv8m.main-none-eabihf/debug/nemo_firmware";
    let file = fs::File::open(&path).unwrap();
    let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };
    let object = object::File::parse(&*mmap).unwrap();
    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
        match object.section_by_name(id.name()) {
            Some(ref section) => Ok(section
                .uncompressed_data()
                .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
            None => Ok(borrow::Cow::Borrowed(&[][..])),
        }
    };

    // Load all of the sections.
    let dwarf_cow = gimli::Dwarf::load(&load_section)?;

    // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
    let borrow_section: &dyn for<'a> Fn(
        &'a borrow::Cow<[u8]>,
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_cow.borrow(&borrow_section);

    // Iterate over the compilation units.
    let mut iter = dwarf.units();

    let mut targets: Vec<TraceData> = Vec::new();
    let mut target_section_index = None;

    // Find the target section index
    for (index, section) in object.sections().enumerate() {
        if let Ok(name) = section.name() {
            if name == "._trace_point" {
                target_section_index = Some(index);
                break;
            }
        }
    }

    if let Some(idx) = target_section_index {
        // Process symbols in '.symtab' section
        for symbol in object.symbols() {
            if symbol.section_index() == Some(object::SectionIndex(idx)) {
                targets.push(TraceData {
                    id: symbol.address(),
                    name: symbol.name()?.to_string(),
                    path: None,
                    file_name: None,
                    line: None,
                });
            }
        }
    }

    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;

        let file_list_buf: Vec<(String, String)> = unit
            .line_program
            .as_ref()
            .unwrap()
            .header()
            .file_names()
            .iter()
            .map(|x| {
                (
                    dwarf
                        .attr_string(
                            &unit,
                            x.directory(unit.line_program.as_ref().unwrap().header())
                                .unwrap(),
                        )
                        .unwrap()
                        .to_string()
                        .unwrap()
                        .to_owned(),
                    dwarf
                        .attr_string(&unit, x.path_name())
                        .unwrap()
                        .to_string()
                        .unwrap()
                        .to_owned(),
                )
            })
            .collect();

        let mut iter = unit.entries();

        while let Some((_, die)) = iter.next_dfs()? {
            if let Some(dw_at_name) = die.attr(gimli::DW_AT_linkage_name)? {
                if let Some(dw_at_name) = dw_at_name.string_value(&dwarf.debug_str) {
                    for trace_data in targets.iter_mut() {
                        if trace_data.name == dw_at_name.to_string().unwrap() {
                            if let gimli::AttributeValue::FileIndex(x) =
                                die.attr(gimli::DW_AT_decl_file).unwrap().unwrap().value()
                            {
                                let path_file_name = file_list_buf[(x - 1) as usize].clone();
                                trace_data.path = Some(path_file_name.0);
                                trace_data.file_name = Some(path_file_name.1);
                                trace_data.line = die
                                    .attr(gimli::DW_AT_decl_line)
                                    .unwrap()
                                    .unwrap()
                                    .value()
                                    .udata_value();
                            }
                        }
                    }
                }
            }
        }
    }

    println!("{:?}", targets);

    Ok(())
}
