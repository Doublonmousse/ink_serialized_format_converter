//  ---------------------------------------------------------------------------------
//  Copyright (c) Microsoft Corporation.  All rights reserved.
// 
//  The MIT License (MIT)
// 
//  Permission is hereby granted, free of charge, to any person obtaining a copy
//  of this software and associated documentation files (the "Software"), to deal
//  in the Software without restriction, including without limitation the rights
//  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//  copies of the Software, and to permit persons to whom the Software is
//  furnished to do so, subject to the following conditions:
// 
//  The above copyright notice and this permission notice shall be included in
//  all copies or substantial portions of the Software.
// 
//  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
//  THE SOFTWARE.
//  ---------------------------------------------------------------------------------

using System;
using System.Collections.Generic;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;

using Windows.Storage.Streams;
using Windows.UI.Input.Inking;
using System.Diagnostics;
using System.Text.Json;
using Windows.UI;
using System.Linq;
using System.Drawing;
using System.IO;
using Windows.Storage;

// The Blank Page item template is documented at https://go.microsoft.com/fwlink/?LinkId=402352&clcid=0x409

namespace Ink_Store
{

    /// All classes to store stroke information 
    public class PointJSON
    {
        public double X { get; set; }
        public double Y { get; set; }
        public float pressure { get; set; }
    }

    public class SingleStrokeDataJSON
    {
        public List<PointJSON> points { get; set; }
        public Windows.UI.Color color { get; set; }
        public Windows.Foundation.Size size { get; set; }
        public bool ignorePressure { get; set; }
    }

    public class StrokeDataJSON
    {
        public List<SingleStrokeDataJSON> strokedata { get; set; }
    }

    /// <summary>
    /// An empty page that can be used on its own or navigated to within a Frame.
    /// </summary>
    public sealed partial class MainPage : Page
    {
        /// <summary>
        /// Our application's single UI page.
        /// </summary>
        public MainPage()
        {
            this.InitializeComponent();

            // Set supported inking device types.
            inkCanvas.InkPresenter.InputDeviceTypes =
                Windows.UI.Core.CoreInputDeviceTypes.Mouse |
                Windows.UI.Core.CoreInputDeviceTypes.Pen;

            // Listen for button click to initiate load.
            btnLoad.Click += btnLoad_Click;
            // Listen for button click to clear ink canvas.
            btnClear.Click += btnClear_Click;
        }

        /// <summary>
        /// Clear ink canvas of all ink strokes.
        /// </summary>
        /// <param name="sender">Source of the click event</param>
        /// <param name="e">Event args for the button click routed event</param>
        private void btnClear_Click(object sender, RoutedEventArgs e)
        {
            inkCanvas.InkPresenter.StrokeContainer.Clear();
        }


        /// <summary>
        /// Load ink data from a file, deserialize it, and add it to ink canvas.
        /// </summary>
        /// <param name="sender">Source of the click event</param>
        /// <param name="e">Event args for the button click routed event</param>
        private async void btnLoad_Click(object sender, RoutedEventArgs e)
        {
            // Let users choose their ink file using a file picker.
            // Initialize the picker.
            Windows.Storage.Pickers.FileOpenPicker openPicker =
                new Windows.Storage.Pickers.FileOpenPicker();
            openPicker.SuggestedStartLocation =
                Windows.Storage.Pickers.PickerLocationId.DocumentsLibrary;
            openPicker.FileTypeFilter.Add(".gif");
            // Show the file picker.
            var files = await openPicker.PickMultipleFilesAsync();
            // User selects a file and picker returns a reference to the selected file.
            if (files.Count > 0)
            {
                foreach (StorageFile file in files)
                {
                    // Open a file stream for reading.
                    IRandomAccessStream stream = await file.OpenAsync(Windows.Storage.FileAccessMode.Read);
                    // Read from file.
                    using (var inputStream = stream.GetInputStreamAt(0))
                    {
                        await inkCanvas.InkPresenter.StrokeContainer.LoadAsync(stream);
                    }
                    stream.Dispose();
                    btnSave_Click(file);
                }

                await Windows.System.Launcher.LaunchFolderAsync(ApplicationData.Current.LocalFolder);
            }
            // User selects Cancel and picker returns null.
            else
            {
                // Operation cancelled.
            }
        }

        /// <summary>
        /// Get ink data from ink canvas, serialize it, and save it to a file.
        /// </summary>
        private async void btnSave_Click(Windows.Storage.StorageFile file_start)
        {
            // Get all strokes on the InkCanvas.
            IReadOnlyList<InkStroke> currentStrokes = inkCanvas.InkPresenter.StrokeContainer.GetStrokes();

            List<SingleStrokeDataJSON> strokesList = new List<SingleStrokeDataJSON>();

            foreach (InkStroke stroke in currentStrokes)
            {
                //tranforms : we suppose it's always the identity
                //Debug.WriteLine("stroke transformation : %s", stroke.PointTransform.IsIdentity.ToString());
                //Debug.WriteLine(stroke.DrawingAttributes.PenTipTransform.ToString()); //transforms ?

                List<PointJSON> points = new List<PointJSON>();

                //individual ink data points
                foreach (InkPoint inkpoint in stroke.GetInkPoints())
                {
                    points.Add(
                        new PointJSON
                        {
                            X = inkpoint.Position.X,
                            Y = inkpoint.Position.Y,
                            pressure = inkpoint.Pressure,
                        }
                        );
                }

                strokesList.Add(
                    new SingleStrokeDataJSON
                    {
                        points = points,
                        color = stroke.DrawingAttributes.Color,
                        size = stroke.DrawingAttributes.Size,
                        ignorePressure = stroke.DrawingAttributes.IgnorePressure,
                    }
                    );
            }

            // consolidate into a StrokeDataJSON
            StrokeDataJSON allstrokes = new StrokeDataJSON
            {
                strokedata = strokesList
            };

            // same the file at the same place under the same filename with a .json extension
            string filename_s = file_start.Name.Split('.')[0] + ".json";
            string path_output = System.IO.Path.Combine(Windows.Storage.ApplicationData.Current.LocalFolder.Path.ToString(), filename_s);

            // Write the ink strokes to the output stream.
            string json_serialize = JsonSerializer.Serialize(allstrokes);
            File.WriteAllText(path_output, json_serialize);
        }
    }
}
