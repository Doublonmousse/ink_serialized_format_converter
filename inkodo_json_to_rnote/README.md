# What

Convert inkodo files to rnote files. To do so, you first have to extract the stroke content from the inkodo backup (by using the other part of this project, the ink_serialized_format_converter, a UWP windows-only app).

# Structure 

```
|-- InkJSON
|----- name_of_page.json
```
This is the output of the ink_serialized_format_converter on the Inks .gif files (Load all of the .gif files from the Inks folder and create the `InkJSON` folder with all of the json that are outputed from the ink_serialized_format_converter)
```
|-- Inks
|---- page_id.obj : Contains the objects in a page
```
As a txt file, one object per line
Format
object_filename;*;width;height;*;*;*;*;*;*;*;x;y;*;*;*;*
Not sure what all of the element are for the rest (not used)
```
|-- Objects
|---- id.objtxt or id_objimg.image_filetype
|-- book_id.db : ID, Title to take
|-- page_id.db : ID, BookRef (use the id from earlier to collate), Created (use the date to sort a book), CanvasWidth, CanvasHeight (in px), CanvasStyle (GRID), CanvasStyleGrid (grid size in px), LinesColor, DisplayOrder
```
(the db files can be parsed as json)

# Usage : 

```bash
cargo run root_folder 
```
And will output `.rnote` files
