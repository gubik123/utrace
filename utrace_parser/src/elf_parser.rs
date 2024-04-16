use crate::trace_point::{TracePointDataWithLocation, TracePointId};
use anyhow::{bail, Context, Result};
use object::{Object, ObjectSection, ObjectSymbol};
use std::{borrow, collections::HashMap, io::Read, path::Path};

pub fn parse<T>(elf_file: T) -> Result<HashMap<TracePointId, TracePointDataWithLocation>>
where
    T: AsRef<Path> + std::fmt::Debug,
{
    let mut file = std::fs::File::open(elf_file.as_ref())
        .with_context(|| format!("Unable to open file {:?}", elf_file))?;

    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data)
        .context("Unable to read elf file")?;

    let object =
        object::File::<&[u8]>::parse(file_data.as_ref()).context("Unable to parse elf file")?;

    let trace_points_section = object
        .section_by_name(crate::TRACE_POINT_SECTION_NAME)
        .context("Unable to find utrace info in the provided elf file")?;
    let trace_points_section_index = trace_points_section.index();

    let mut trace_point_list: HashMap<String, TracePointId> = HashMap::new();

    for symbol in object.symbols() {
        if symbol.section_index() == Some(trace_points_section_index) {
            let symbol_addr = symbol.address();
            if symbol_addr as usize > crate::MAX_TRACE_POINTS {
                bail!("Provided elf file contains too many trace points (check linker script)");
            }
            trace_point_list.insert(
                symbol
                    .name()
                    .with_context(|| {
                        format!("Invalid trace symbol metadata for id={}", symbol.address())
                    })?
                    .to_string(),
                symbol_addr as TracePointId,
            );
        }
    }

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

    let dwarf_cow =
        gimli::Dwarf::load(&load_section).context("Unable to load DWARF info from elf")?;

    let borrow_section: &dyn for<'a> Fn(
        &'a borrow::Cow<[u8]>,
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(section, endian);

    let dwarf = dwarf_cow.borrow(&borrow_section);

    let mut iter = dwarf.units();

    let mut ret = HashMap::new();

    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;

        // Vector of (path, filename) pairs for the current unit
        let file_list_buf: Vec<(Option<String>, Option<String>)> = {
            let lp_header = unit.line_program.as_ref().map(|lp| lp.header());

            if let Some(lp_header) = lp_header {
                lp_header
                    .file_names()
                    .iter()
                    .map(|x| {
                        (
                            {
                                let dir = x.directory(lp_header);
                                if let Some(dir) = dir {
                                    dwarf
                                        .attr_string(&unit, dir)
                                        .and_then(|a| a.to_string())
                                        .map(|s| s.to_owned())
                                        .ok()
                                } else {
                                    None
                                }
                            },
                            dwarf
                                .attr_string(&unit, x.path_name())
                                .and_then(|a| a.to_string())
                                .map(|s| s.to_owned())
                                .ok(),
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };

        let mut iter = unit.entries();

        while let Some((_, die)) = iter.next_dfs().context("Malformed DWARF info in elf")? {
            if let Some(dw_at_name) = die
                .attr(gimli::DW_AT_linkage_name)
                .context("Malformed DWARF info in elf")?
            {
                if let Some(dw_at_name) = dw_at_name.string_value(&dwarf.debug_str) {
                    let trace_point_data = dw_at_name
                        .to_string()
                        .context("Malformed DWARF info in elf")?;
                    let trace_point_idx = trace_point_list.get(trace_point_data);
                    if let Some(trace_point_idx) = trace_point_idx {
                        let parsed_data =
                            serde_json::from_str(trace_point_data).with_context(|| {
                                format!("Cannot parse tracepoint {} metadata", trace_point_idx)
                            })?;

                        let mut ret_item = TracePointDataWithLocation {
                            info: parsed_data,
                            path: None,
                            file_name: None,
                            line: None,
                        };

                        if let Some(gimli::AttributeValue::FileIndex(x)) = die
                            .attr(gimli::DW_AT_decl_file)
                            .context("Malformed DWARF info in elf")?
                            .map(|a| a.value())
                        {
                            let path_file_name = file_list_buf[(x - 1) as usize].clone();
                            ret_item.path = path_file_name.0;
                            ret_item.file_name = path_file_name.1;
                            ret_item.line = die
                                .attr(gimli::DW_AT_decl_line)
                                .context("Malformed DWARF info in elf")?
                                .map(|a| a.value())
                                .and_then(|a| a.udata_value());
                        }

                        ret.insert(*trace_point_idx, ret_item);
                    }
                }
            }
        }
    }

    Ok(ret)
}
