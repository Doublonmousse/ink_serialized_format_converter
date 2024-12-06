Utilities to convert inkodo files to rnote.

This is done in several steps

+ Export from inkodo (create a backup) and extract it. You end up with several folders (with images and .gif files)
+ Get stroke information out from strokes (using `ink_serialized_to_json`). This is a UWP Windows only application (to be able to use directly the only library that currently exists that can read these files). This allows you to add to the previous backup json files containing the stroke information
+ Convert all of this into rnote notes (loads the different json and create rnote files coresponding to inkodo books -but not subjects-)

Beware: No guarantee that this will work in all cases. This was more of a tool that coincidentally worked with my own files so not everything is supported, as not everything was tested (text is not for example)