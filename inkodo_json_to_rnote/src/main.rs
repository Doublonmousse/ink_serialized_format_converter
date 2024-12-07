extern crate nalgebra as na;
use anyhow::bail;
use rnote_compose::penpath::Element;
use rnote_compose::shapes::Rectangle;
use rnote_compose::style::smooth::SmoothOptions;
use rnote_compose::style::PressureCurve;
use rnote_compose::Color;
use rnote_compose::PenPath;
use rnote_engine::document::background::PatternStyle;
use rnote_engine::document::Layout;
use rnote_engine::store::chrono_comp::StrokeLayer;
use rnote_engine::strokes::BrushStroke;
use rnote_engine::strokes::Stroke;
use rnote_engine::Engine;
use serde::{de::Error, Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    smol::block_on(async { load_into_rnote().await })
}

async fn get_root_folder() -> anyhow::Result<PathBuf> {
    let path_str = std::env::args().nth(1).expect("no pattern given");
    let root_folder = PathBuf::from(path_str);
    if !(root_folder.exists() && root_folder.is_dir()) {
        bail!(anyhow::anyhow!(
            "could not find the path provided or is not a folder"
        ));
    } else {
        Ok(root_folder)
    }
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct BookEntry {
    ID: u64,
    Title: String,
    #[serde(skip_deserializing)]
    Pages: Vec<PageEntry>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct PageEntry {
    ID: u64,
    BookRef: u64,
    #[serde(deserialize_with = "from_hex")]
    Color: (u8, u8, u8, u8),
    DisplayOrder: u64,
    CanvasWidth: f64,
    CanvasHeight: f64,
    CanvasStyle: String,
    CanvasStyleGrid: f64,
    #[serde(deserialize_with = "from_hex")]
    LinesColor: (u8, u8, u8, u8),
}

fn from_hex<'de, D>(deserializer: D) -> Result<(u8, u8, u8, u8), D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    if s.len() == 9 {
        let a = u8::from_str_radix(&s[1..=2], 16).map_err(D::Error::custom)?;
        let r = u8::from_str_radix(&s[3..=4], 16).map_err(D::Error::custom)?;
        let g = u8::from_str_radix(&s[5..=6], 16).map_err(D::Error::custom)?;
        let b = u8::from_str_radix(&s[7..=8], 16).map_err(D::Error::custom)?;
        return Ok((r, g, b, a));
    } else {
        Err(D::Error::custom(String::from("couldn't parse hex")))
    }
}

async fn get_books_and_pages() -> anyhow::Result<(PathBuf, HashMap<u64, BookEntry>)> {
    let root_folder = get_root_folder().await?;
    if !(root_folder.exists() && root_folder.is_dir()) {
        bail!(anyhow::anyhow!(
            "could not find the path provided or is not a folder"
        ));
    } else {
        println!("{:?}", root_folder.canonicalize()?);
        let mut unnamed_pages_counter: u64 = 0;
        let book_iterator = get_iterator(&root_folder, String::from("db"), String::from("book_"));
        let mut book_collection: HashMap<u64, BookEntry> = HashMap::new();

        for book in book_iterator {
            let fs = fs::read_to_string(book.path())?;
            let dic: BookEntry = serde_json::from_str(&fs)?;
            println!("dic {:?}", dic);
            book_collection.insert(dic.ID, dic);
        }
        let page_iterator = get_iterator(&root_folder, String::from("db"), String::from("page"));
        for page in page_iterator {
            let fs = fs::read_to_string(page.path())?;
            let page: PageEntry =
                serde_json::from_str(&fs).map_err(|_| anyhow::anyhow!("couldn't parse page"))?;
            println!("{:?}", page);
            // rattach to dic
            if page.BookRef == 0 {
                book_collection.insert(
                    unnamed_pages_counter,
                    BookEntry {
                        ID: unnamed_pages_counter,
                        Title: format!("Unnamed_{unnamed_pages_counter}"),
                        Pages: vec![page],
                    },
                );
                unnamed_pages_counter += 1;
            } else {
                if book_collection.contains_key(&page.BookRef) {
                    let current_value = book_collection.get_mut(&page.BookRef).unwrap();
                    current_value.Pages.push(page);
                    // mutate in place
                }
            }
        }

        // sort pages per creation date (the page number is invalid on my files and always 0 !!)
        for (_, book) in book_collection.iter_mut() {
            book.Pages
                .sort_by(|a, b| a.DisplayOrder.cmp(&b.DisplayOrder));
        }

        Ok((root_folder, book_collection))
    }
}

async fn load_into_rnote() -> anyhow::Result<()> {
    let (root_folder, book_collection) = get_books_and_pages().await?;
    println!("{:?}", book_collection);
    for (_, book) in book_collection.into_iter() {
        load_book(book, &root_folder).await?;
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct StrokeElement {
    X: f64,
    Y: f64,
    pressure: f64,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct ColorData {
    A: u8,
    R: u8,
    G: u8,
    B: u8,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SizeData {
    Width: f64,
    Height: f64,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct StrokeData {
    points: Vec<StrokeElement>,
    color: ColorData,
    size: SizeData,
    ignorePressure: bool,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct PageStrokeJSON {
    strokedata: Vec<StrokeData>,
}

async fn load_book(book: BookEntry, root_folder: &PathBuf) -> anyhow::Result<()> {
    let mut engine = Engine::default();

    let (mut height, width) = book
        .Pages
        .iter()
        .fold((0.0 as f64, 0.0 as f64), |acc, page| {
            (acc.0.max(page.CanvasHeight), acc.1.max(page.CanvasWidth))
        });
    println!("{:?} {:?}", height, width);

    let is_grid = book
        .Pages
        .iter()
        .fold(false, |acc, page| acc || page.CanvasStyle == "GRID");

    height = if is_grid {
        let grid_size = book
            .Pages
            .iter()
            .fold(0.0 as f64, |_acc, page| page.CanvasStyleGrid);

        let grid_color = book
            .Pages
            .iter()
            .fold((0 as u8, 0 as u8, 0 as u8, 0 as u8), |_acc, page| {
                page.LinesColor
            });

        engine.document.background.pattern_color = Color::new(
            grid_color.0 as f64 / 255.0,
            grid_color.1 as f64 / 255.0,
            grid_color.2 as f64 / 255.0,
            grid_color.3 as f64 / 255.0,
        );

        engine.document.layout = Layout::ContinuousVertical;

        engine.document.background.pattern = PatternStyle::Grid;
        engine.document.background.pattern_size = na::Vector2::new(grid_size, grid_size);
        (height / grid_size).ceil() * grid_size
    } else {
        height
    };

    engine.document.x = 0.0;
    engine.document.y = 0.0;
    engine.document.width = width;
    engine.document.height = height;
    engine.document.format.set_height(height);
    engine.document.format.set_width(width);

    let color_background = book
        .Pages
        .iter()
        .fold((0 as u8, 0 as u8, 0 as u8, 0 as u8), |_acc, page| {
            page.Color
        });
    engine.document.background.color = Color::new(
        color_background.0 as f64 / 255.0,
        color_background.1 as f64 / 255.0,
        color_background.2 as f64 / 255.0,
        color_background.3 as f64 / 255.0,
    );
    let mut strokes_collect : Vec<(Stroke,Option<StrokeLayer>)> = vec![];

    // iterate over pages
    for (page_num, page) in book.Pages.iter().enumerate() {
        // get id
        let id = format!("InkJSON/{}.json", page.ID);
        let path_stroke = root_folder.join(id);
        if path_stroke.is_file() {
            // load the file content
            let fs = fs::read_to_string(path_stroke.as_path())?;
            let strokedata: PageStrokeJSON = serde_json::from_str(&fs)?;

            // iterate over the strokedata
            for stroke in strokedata.strokedata {
                // style options
                let mut smooth_options = SmoothOptions::default();

                smooth_options.stroke_color = Some(Color::new(
                    stroke.color.R as f64 / 255.0,
                    stroke.color.G as f64 / 255.0,
                    stroke.color.B as f64 / 255.0,
                    stroke.color.A as f64 / 255.0,
                ));
                smooth_options.stroke_width = stroke.size.Height.max(stroke.size.Width);

                if stroke.ignorePressure {
                    smooth_options.pressure_curve = PressureCurve::Const;
                } else {
                    smooth_options.pressure_curve = PressureCurve::Linear;
                }

                let penpath =
                    PenPath::try_from_elements(stroke.points.into_iter().map(|stroke_el| {
                        Element::new(
                            na::vector![stroke_el.X, stroke_el.Y + (page_num as f64) * height],
                            stroke_el.pressure,
                        )
                    }))
                    .ok_or_else(|| {
                        anyhow::anyhow!("Could not generate pen path from coordinates vector")
                    })?;

                let new_stroke = BrushStroke::from_penpath(
                    penpath,
                    rnote_compose::Style::Smooth(smooth_options),
                );

                let layer = StrokeLayer::UserLayer(0);
                strokes_collect.push(
                    (Stroke::BrushStroke(new_stroke), Some(layer))
                );
            }

            // see if there is any images to insert as well
            let object_file = root_folder.join(format!("Inks/{}.obj", page.ID));
            let fs = fs::read_to_string(object_file)?;
            for line in fs.lines() {
                if line.len() > 0 {
                    let objects = line.split(';').collect::<Vec<&str>>();
                    let filename = objects[0];
                    let mut width_img = (objects[2]).parse::<f64>()?;
                    let mut height_img = (objects[3]).parse::<f64>()?;
                    let x_img = (objects[11]).parse::<f64>()?;
                    let y_img = (objects[12]).parse::<f64>()?;

                    //check that the file exists
                    let path_file = root_folder.join(format!("Objects/{}", filename));
                    if path_file.exists() && path_file.extension().is_some_and(|x| x == "png") {
                        // how to load the image ?
                        println!("found image, {:?}, path {:?}", objects, path_file);
                        // need to generate the image manually
                        // similarly to what's done in load_in_bitmapimage_bytes
                        // load the file bytes
                        let bytes = std::fs::read(&path_file)?;
                        let mut bitmapimage = engine
                            .generate_bitmapimage_from_bytes(
                                na::Vector2::new(x_img, y_img + (page_num as f64) * height),
                                bytes,
                                false,
                            )
                            .await??;

                        let ratio = bitmapimage.rectangle.cuboid.half_extents[1]
                            / bitmapimage.rectangle.cuboid.half_extents[0];
                        if width_img.is_nan() {
                            width_img = height_img / ratio;
                        } else if height_img.is_nan() {
                            height_img = width_img * ratio;
                        }
                        // modify the size
                        bitmapimage.rectangle = Rectangle::from_corners(
                            na::Vector2::new(x_img, y_img + (page_num as f64) * height),
                            na::Vector2::new(
                                x_img + width_img,
                                y_img + (page_num as f64) * height + height_img,
                            ),
                        );

                        strokes_collect.push(
                            (Stroke::BitmapImage(bitmapimage), None)
                        );
                    }
                }
            }
        }
    }

    // push all strokes to the engine
    let _ = engine.import_generated_content(strokes_collect,false);

    let bytes = engine.save_as_rnote_bytes(book.Title.clone()).await??;

    let mut add_filetype = book.Title;
    add_filetype.push_str(".rnote");

    let mut fh = File::create(Path::new(&add_filetype))?;
    fh.write_all(&bytes)?;
    fh.sync_all()?;

    Ok(())
}

fn get_iterator(
    root_folder: &PathBuf,
    filetype: String,
    start_el: String,
) -> impl Iterator<Item = std::fs::DirEntry> {
    root_folder
        .read_dir()
        .expect("expected dir")
        .filter(move |x| {
            x.is_ok()
                && x.as_ref().unwrap().metadata().is_ok()
                && x.as_ref().unwrap().metadata().unwrap().is_file()
                && x.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == filetype.as_str())
                && x.as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .is_some_and(|filename| filename.starts_with(&start_el))
        })
        .map(|x| x.unwrap())
        .into_iter()
}
