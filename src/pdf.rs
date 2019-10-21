// https://itchyny.hatenablog.com/entry/2015/09/16/100000
// texttopdf.hsをRustに移植したもの。
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum PDFElem {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
    Name(String),
    Ref(i64),
    Array(Vec<PDFElem>),
    Dict(Vec<Dict>),
    Stream(String),
}

#[derive(Debug, Clone)]
pub struct Dict {
    key: String,
    value: PDFElem,
}

fn new_dict(key:&str, value:PDFElem) -> Dict {
    Dict {key:key.to_string(), value:value}
}

pub fn render_elem(e: &PDFElem) -> String {
    let escape_string = |s: &str| -> String {
        s.replace(r#"\"#, r#"\\"#).as_str()
            .replace(r#"("#, r#"\("#).as_str()
            .replace(r#")"#, r#"\)"#)
    };

    let escape_name = |s: &str| -> String {
        s.chars().map(|c: char| {
            if c < '!' || c > '~' || c == '#' {
                "#".to_string() + format!("{:02x}", c as u32).as_str()
            } else {
                c.to_string()
            }
        }).collect::<Vec<String>>().join("")
    };
    match e {
        PDFElem::Null => "null".to_string(),
        PDFElem::Bool(true) => "true".to_string(),
        PDFElem::Bool(false) => "false".to_string(),
        PDFElem::Int(i) => i.to_string(),
        PDFElem::String(s) => "(".to_string() + escape_string(s.as_str()).as_str() + ")",
        PDFElem::Name(s) => format!("/{}", escape_name(s)),
        PDFElem::Ref(n) => n.to_string() + " 0 R",
        PDFElem::Array(l) => format!("[{}]", l.iter().map(|o: &PDFElem| render_elem(&o)).collect::<Vec<String>>().join(" ")),
        PDFElem::Dict(dict) => {
            let mut result = "<<\n".to_string();
            for kv in dict {
                result = result + "/" + kv.key.as_str() + " " + render_elem(&kv.value).as_str() + "\n";
            }
            result + ">>"
        },
        PDFElem::Stream(s) => {
            let mut result = "<<\n".to_string();
            result = result + "/Length " + s.len().to_string().as_str();
            result = result + "\n>>\nstream\n";
            result = result + s.as_str();
            result = result + "\nendstream";
            result
        }
    }
}

#[derive(Debug, Clone)]
pub struct PDFObj {
    pub n: i64,
    pub elem: PDFElem,
}

pub fn render_obj(obj: &PDFObj) -> String {
    (obj.n.to_string() + " 0 obj\n" + render_elem(&obj.elem).as_str() + "\nendobj\n").to_string()
}

pub fn render_header(n: u32, m:u32) -> String {
    // format!(
    //     "%PDF-{}.{}\n%{}\n", n, m,
    //     String::from_utf8_lossy(&vec![0xe2, 0xe3, 0xcf, 0xd3]) // 2行目のバイナリ文字列。
    // )
    unsafe {
        format!(
            "%PDF-{}.{}\n%{}\n", n, m,
            String::from_utf8_unchecked(vec![0xe2, 0xe3, 0xcf, 0xd3]) // 2行目のバイナリ文字列。
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PdfXrefEntryUse {
    FreeEntry,
    InUseEntry,
}

#[derive(Debug, Clone, Copy)]
pub struct PDFXrefEntry {
    offset: i64,
    generation: i64,
    entry_use: PdfXrefEntryUse,
}

type PdfXref = Vec<PDFXrefEntry>;

pub fn render_xref(xs:PdfXref) -> String {
    format!(
        "xref\n0 {}\n{}",
        xs.len()+1,
        xs.iter()
        .map(|&a| render_xref_entry(a))
        .collect::<Vec<String>>().join(""))
}

pub fn render_xref_entry(entry:PDFXrefEntry) -> String {
    let l = vec![
        format!("{:010}", entry.offset),
        format!("{:05}", entry.generation),
        render_xref_entry_use(entry.entry_use),
        "\n".to_string(),
    ];
    l.join(" ")
}

pub fn render_xref_entry_use(u:PdfXrefEntryUse) -> String {
    match u {
        PdfXrefEntryUse::FreeEntry => "f".to_string(),
        PdfXrefEntryUse::InUseEntry => "n".to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PdfTrailer {
    root: i64,
    size: i64,
    startxref: i64,
}

#[derive(Debug, Clone)]
pub struct PDFFile {
    version: (u32, u32),
    catalog_number: i64,
    objlist: Vec<PDFObj>,
}

pub fn render_trailer(trailer:PdfTrailer) -> String {
    let dict = vec![
        new_dict("Size", PDFElem::Int(trailer.size)),
        new_dict("Root", PDFElem::Ref(trailer.root)),
    ];
    format!(
        "trailer\n{}\nstartxref\n{}\n%%EOF\n",
        render_elem(&PDFElem::Dict(dict)),
        render_elem(&PDFElem::Int(trailer.startxref)))
}

pub fn render_pdf(pdf_file:PDFFile) -> String {
    let header = render_header(pdf_file.version.0, pdf_file.version.1);
    let objects = pdf_file.objlist.iter().map(|x| render_obj(x)).collect::<Vec<String>>();
    let mut prev = header.len();
    let mut offsets = vec![];
    for i in 0..objects.len() {
        prev = objects[i].len() + prev;
        offsets.push(prev as i64);
    }

    let mut pdf_xref = vec![];
    pdf_xref.push(PDFXrefEntry {offset:0, generation:65535, entry_use:PdfXrefEntryUse::FreeEntry});
    if offsets.len() > 1 {
        for i in 0..offsets.len()-1 {
            pdf_xref.push(PDFXrefEntry {offset:offsets[i], generation:0, entry_use:PdfXrefEntryUse::InUseEntry});
        }
    }
    let xref = render_xref(pdf_xref);

    let trailer = render_trailer(PdfTrailer{
        root:pdf_file.catalog_number,
        size:pdf_file.objlist.len() as i64 +1,
        startxref:offsets[offsets.len()-1],
    });

    format!("{}{}{}{}", header, objects.join(""), xref, trailer)
}

pub fn text_to_pdf(texts: Vec<Vec<String>>) -> PDFFile {
    let n = texts.len();
    let m = 4;

    let dict = vec![
        new_dict("Type", PDFElem::Name("Catalog".to_string())),
        new_dict("Pages", PDFElem::Ref(2)),
    ];
    let catalog = PDFObj {n: 1, elem: PDFElem::Dict(dict)};

    let kids = (m..m+n).map(|i| PDFElem::Ref(i as i64)).collect();
    let dict = vec![
        new_dict("Type", PDFElem::Name("Pages".to_string())),
        new_dict("Kids", PDFElem::Array(kids)),
        new_dict("Count", PDFElem::Int(n as i64)),
    ];
    let top_page = PDFObj {n: 2, elem: PDFElem::Dict(dict)};

    let font_dict_f0 = vec![
        new_dict("Type", PDFElem::Name("Font".to_string())),
        new_dict("BaseFont", PDFElem::Name("Times-Roman".to_string())),
        new_dict("Subtype", PDFElem::Name("Type1".to_string())),
    ];
    let font_dict = vec![
        new_dict("F0", PDFElem::Dict(font_dict_f0)),
    ];
    let dict = vec![
        new_dict("Font", PDFElem::Dict(font_dict)),
    ];
    let font = PDFObj {n: 3, elem: PDFElem::Dict(dict)};

    let mut pages = vec![];
    for i in m..m+n {
        let d = vec![
            new_dict("Type", PDFElem::Name("Page".to_string())),
            new_dict("Parent", PDFElem::Ref(2)),
            new_dict("Resources", PDFElem::Ref(3)),
            new_dict("MediaBox", PDFElem::Array(vec![
                PDFElem::Int(0),
                PDFElem::Int(0),
                PDFElem::Int(595),
                PDFElem::Int(842),
            ])),
            new_dict("Contents", PDFElem::Ref((i+n) as i64)),
        ];
        pages.push(PDFObj {n: i as i64, elem: PDFElem::Dict(d)});
    }

    let mut contents = vec![];
    for (i, text) in (m+n..m+n*2).zip(texts.iter()) {
        let stream0 = "1. 0. 0. 1. 50. 770. cm\nBT\n/F0 12 Tf\n16 TL\n".to_string();
        let stream1 = text.iter()
                        .map(|t| render_elem(&PDFElem::String(t.to_string())) + " Tj T*\n")
                        .collect::<String>(); 

        contents.push(PDFObj{n:i as i64, elem:PDFElem::Stream(format!("{}{}ET", stream0, stream1))});
    }

    let mut objlist = vec![];
    objlist.push(catalog);
    objlist.push(top_page);
    objlist.push(font);
    objlist.extend(pages);
    objlist.extend(contents);
    PDFFile {
        version: (1, 7),
        catalog_number: 1,
        objlist: objlist,
    }
}
