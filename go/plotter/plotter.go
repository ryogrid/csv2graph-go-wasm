package plotter

import (
	// ... other imports
	"bytes" // Make sure bytes is imported
	"encoding/base64"
	"encoding/csv"
	"fmt"
	"image"
	"image/color"
	"image/png"
	"math"
	"strconv"
	"strings"

	"github.com/fogleman/gg"
)

// PlotOptions holds the configuration for generating the plot.
type PlotOptions struct {
	Columns  []string `json:"columns"`          // Columns to plot
	MaxRange float64  `json:"maxRange"`         // Max X value, plot points <= this (optional, <= 0 means no limit)
	Size     string   `json:"size,omitempty"`   // Output image size as "WIDTHxHEIGHT" string (e.g., "800x600")
	Width    int      `json:"width"`            // Output image width (overridden by Size if provided)
	Height   int      `json:"height"`           // Output image height (overridden by Size if provided)
	Skip     int      `json:"skip"`             // Data thinning (plot every Nth point, default=1)
	XData    bool     `json:"xdata"`            // When true, CSV has X-axis values in first column
	XScale   string   `json:"xscale,omitempty"` // Map X values to range "START,END" (optional)
	Title    string   `json:"title"`            // Graph title
}

// GeneratePlot generates a scatter plot image from CSV data and options.
// It returns a base64 encoded PNG string and an error if any occurred.
func GeneratePlot(csvData string, opts PlotOptions) (string, error) {
	// Validate options
	if opts.Skip < 1 {
		opts.Skip = 1
	}
	if opts.Width <= 0 {
		opts.Width = 768 // Default width
	}
	if opts.Height <= 0 {
		opts.Height = 512 // Default height
	}
	if opts.Title == "" {
		opts.Title = "Scatter Plot from CSV"
	}
	if len(opts.Columns) == 0 {
		return "", fmt.Errorf("no columns specified to plot")
	}

	// Use strings.NewReader to treat the string data as a file
	csvReader := csv.NewReader(strings.NewReader(csvData))
	records, err := csvReader.ReadAll()
	if err != nil {
		return "", fmt.Errorf("csv read error: %w", err)
	}
	if len(records) < 2 {
		return "", fmt.Errorf("no data rows found in CSV")
	}

	// Parse header and data rows
	header := records[0]
	dataRows := records[1:]

	// Generate X column if xdata=false
	var xIndex int = 0 // Default to first column if xdata=true
	if !opts.XData {
		// Insert placeholder X column name
		header = append([]string{"_generated_x_"}, header...)
		// Insert sequential numbers as X data
		for i := 0; i < len(dataRows); i++ {
			dataRows[i] = append([]string{strconv.Itoa(i + 1)}, dataRows[i]...)
		}
		// xIndex remains 0
	} else if len(header) == 0 || len(dataRows[0]) == 0 {
		return "", fmt.Errorf("csv requires at least one column when xdata is true")
	}

	// Map column names to indexes
	colIndexMap := make(map[string]int)
	for i, h := range header {
		colIndexMap[h] = i
	}

	// Check if all requested columns exist
	var validColsToPlot []string
	var colIndicesToPlot []int
	for _, colName := range opts.Columns {
		idx, ok := colIndexMap[colName]
		if !ok {
			fmt.Printf("Warning: Column '%s' not found in CSV header, skipping.\n", colName) // Log warning
		} else {
			validColsToPlot = append(validColsToPlot, colName)
			colIndicesToPlot = append(colIndicesToPlot, idx)
		}
	}

	if len(validColsToPlot) == 0 {
		return "", fmt.Errorf("none of the specified columns were found in the CSV")
	}

	// Filter data by range (if specified)
	var filtered [][]string
	if opts.MaxRange > 0 {
		for _, row := range dataRows {
			if len(row) <= xIndex {
				continue // Skip rows with missing x-value column
			}
			xVal, err := strconv.ParseFloat(row[xIndex], 64)
			if err == nil && xVal <= opts.MaxRange {
				filtered = append(filtered, row)
			}
		}
	} else {
		filtered = dataRows
	}

	if len(filtered) == 0 {
		return "", fmt.Errorf("no data points remain after filtering by range")
	}

	// Get original data range (X axis)
	origXMin, origXMax := math.Inf(1), math.Inf(-1)
	for _, row := range filtered {
		if len(row) <= xIndex {
			continue
		}
		xVal, err := strconv.ParseFloat(row[xIndex], 64)
		if err == nil {
			origXMin = math.Min(origXMin, xVal)
			origXMax = math.Max(origXMax, xVal)
		}
	}
	if origXMin > origXMax { // Handle case where no valid X values found
		return "", fmt.Errorf("could not determine valid X-axis range from data")
	}
	// Avoid division by zero if all X values are the same
	if origXMin == origXMax {
		origXMax = origXMin + 1.0 // Add a small range
	}

	// Determine plot display range (X axis)
	plotXMin, plotXMax := origXMin, origXMax
	if opts.XScale != "" {
		parts := strings.Split(opts.XScale, ",")
		if len(parts) == 2 {
			s, errS := strconv.ParseFloat(parts[0], 64)
			e, errE := strconv.ParseFloat(parts[1], 64)
			if errS == nil && errE == nil && e > s {
				plotXMin, plotXMax = s, e
			} else {
				fmt.Println("Warning: Invalid xscale format or range, using original data range.")
			}
		} else {
			fmt.Println("Warning: Invalid xscale format, using original data range.")
		}
	}

	// Data thinning
	thinned := [][]string{}
	for i := 0; i < len(filtered); i += opts.Skip {
		thinned = append(thinned, filtered[i])
	}

	if len(thinned) == 0 {
		return "", fmt.Errorf("no data points remain after thinning")
	}

	// Prepare drawing context
	w, h := opts.Width, opts.Height
	dc := gg.NewContext(w, h)
	dc.SetColor(color.White) // Use SetColor instead of SetRGB(1,1,1)
	dc.Clear()
	dc.SetColor(color.Black) // Default drawing color

	// Find Y-axis min, max values from the data to be plotted
	yMin, yMax := math.Inf(1), math.Inf(-1)
	foundYData := false
	for _, idx := range colIndicesToPlot {
		for _, row := range thinned {
			if len(row) <= idx {
				continue
			} // Skip rows with missing column data
			val, err := strconv.ParseFloat(row[idx], 64)
			if err == nil {
				yMin = math.Min(yMin, val)
				yMax = math.Max(yMax, val)
				foundYData = true
			}
		}
	}

	if !foundYData {
		return "", fmt.Errorf("no valid numeric data found in the specified Y columns")
	}
	// Handle case where all Y values are the same
	if yMin == yMax {
		yMax = yMin + 1.0 // Add a small range for plotting
	}

	// Margin and usable area
	margin := 60.0 // Consider making this configurable or dynamic?
	usableW := float64(w) - 2*margin
	usableH := float64(h) - 2*margin
	if usableW <= 0 || usableH <= 0 {
		return "", fmt.Errorf("image size too small for margins")
	}

	// --- Drawing starts ---
	// Draw Axes
	dc.SetLineWidth(1.5)
	dc.DrawLine(margin, float64(h)-margin, float64(w)-margin, float64(h)-margin) // X-axis
	dc.DrawLine(margin, margin, margin, float64(h)-margin)                       // Y-axis
	dc.Stroke()

	// Load default font (gg uses Go's basicfont by default if others aren't loaded)
	// If specific fonts are needed, they must be handled (e.g., embedded)
	// For simplicity, we rely on the default font here.
	// err = dc.LoadFontFace("/path/to/font.ttf", 12) // Example if needed

	// Axis Labels and Ticks
	xTickSteps := 5
	yTickSteps := 5
	labelColor := color.Black

	// X-axis labels & grid
	for i := 0; i <= xTickSteps; i++ {
		ratio := float64(i) / float64(xTickSteps)
		labelVal := plotXMin + ratio*(plotXMax-plotXMin)
		tx := margin + ratio*usableW
		// Tick mark
		dc.SetLineWidth(1)
		dc.DrawLine(tx, float64(h)-margin, tx, float64(h)-margin+5)
		dc.Stroke()
		// Label
		dc.SetColor(labelColor)
		// Use default font size (or set explicitly dc.SetFontSize(10))
		dc.DrawStringAnchored(fmt.Sprintf("%.1f", labelVal), tx, float64(h)-margin+15, 0.5, 0)
		// Grid line
		if i > 0 && i < xTickSteps {
			dc.SetColor(color.RGBA{200, 200, 200, 255}) // Light grey
			dc.SetLineWidth(0.5)
			dc.DrawLine(tx, margin, tx, float64(h)-margin)
			dc.Stroke()
		}
	}

	// Y-axis labels & grid
	for i := 0; i <= yTickSteps; i++ {
		ratio := float64(i) / float64(yTickSteps)
		labelVal := yMin + ratio*(yMax-yMin)
		ty := float64(h) - margin - ratio*usableH
		// Tick mark
		dc.SetLineWidth(1)
		dc.DrawLine(margin-5, ty, margin, ty)
		dc.Stroke()
		// Label
		dc.SetColor(labelColor)
		dc.DrawStringAnchored(fmt.Sprintf("%.1f", labelVal), margin-10, ty, 1, 0.5)
		// Grid line
		if i > 0 && i < yTickSteps {
			dc.SetColor(color.RGBA{200, 200, 200, 255}) // Light grey
			dc.SetLineWidth(0.5)
			dc.DrawLine(margin, ty, float64(w)-margin, ty)
			dc.Stroke()
		}
	}

	// Plot each series
	colors := generateColors(len(validColsToPlot))
	for cidx, colIdx := range colIndicesToPlot {
		points := make([][2]float64, 0, len(thinned)) // Pre-allocate slice capacity

		for _, row := range thinned {
			if len(row) <= xIndex || len(row) <= colIdx {
				continue
			} // Ensure columns exist

			xStr := row[xIndex]
			yStr := row[colIdx]

			xVal, err1 := strconv.ParseFloat(xStr, 64)
			yVal, err2 := strconv.ParseFloat(yStr, 64)

			if err1 == nil && err2 == nil {
				// Normalize X value based on the plot's display range if xscale was used
				normalizedX := xVal
				if opts.XScale != "" {
					// Map original X value to the scaled range [plotXMin, plotXMax]
					if origXMax != origXMin { // Avoid division by zero
						normalizedX = plotXMin + ((xVal-origXMin)/(origXMax-origXMin))*(plotXMax-plotXMin)
					} else {
						normalizedX = plotXMin // If all original X are same, map to start of plot range
					}
				}

				// Calculate screen coordinates: Map data range [plotXMin, plotXMax] and [yMin, yMax] to screen space
				var xx, yy float64
				if plotXMax != plotXMin { // Avoid division by zero
					xx = margin + ((normalizedX-plotXMin)/(plotXMax-plotXMin))*usableW
				} else {
					xx = margin + 0.5*usableW // Center if range is zero
				}

				if yMax != yMin { // Avoid division by zero
					yy = float64(h) - margin - ((yVal-yMin)/(yMax-yMin))*usableH
				} else {
					yy = float64(h) - margin - 0.5*usableH // Center if range is zero
				}

				// Add point if it's within the drawable area margin (slightly wider than axis lines)
				if xx >= margin-1 && xx <= float64(w)-margin+1 && yy >= margin-1 && yy <= float64(h)-margin+1 {
					points = append(points, [2]float64{xx, yy})
				}
			}
		}

		// Draw points and lines for this series
		if len(points) > 0 {
			dc.SetColor(colors[cidx])
			dc.SetLineWidth(1) // Line width for connecting lines

			// Draw connecting lines first
			for i := 1; i < len(points); i++ {
				dc.DrawLine(points[i-1][0], points[i-1][1], points[i][0], points[i][1])
			}
			dc.Stroke() // Stroke all lines at once

			// Draw points on top
			pointRadius := 2.5
			for _, p := range points {
				dc.DrawCircle(p[0], p[1], pointRadius)
				dc.Fill() // Fill the circle
			}
		}
	}

	// Draw Legend
	if len(validColsToPlot) > 0 {
		legendX := float64(w) - margin + 10
		legendY := margin
		boxSize := 10.0
		vSpacing := 18.0 // Vertical spacing between items
		hPadding := 10.0 // Horizontal padding inside legend box
		vPadding := 5.0  // Vertical padding inside legend box
		maxTextWidth := 0.0

		// Calculate legend size needed
		// dc.SetFontSize(10) // <--- この行を削除またはコメントアウト
		// Load default font face if not already loaded (gg might do this automatically)
		// If you haven't called LoadFontFace, gg uses a default font.
		// Let's ensure a font is loaded for measurement, LoadFontFace sets the size
		// If we want a specific size here without external files, it's tricky with default font.
		// We'll proceed assuming the default font is sufficient for measurement.
		for _, name := range validColsToPlot {
			// MeasureString uses the currently set font face in the context
			textWidth, _ := dc.MeasureString(name) // Use MeasureString directly
			if textWidth > maxTextWidth {
				maxTextWidth = textWidth
			}
		}
		legendWidth := hPadding*2 + boxSize + 5 + maxTextWidth                                     // 5 is spacing between box and text
		legendHeight := vPadding*2 + float64(len(validColsToPlot))*vSpacing - (vSpacing - boxSize) // Adjust last spacing

		// Draw legend box background and border
		dc.SetColor(color.RGBA{255, 255, 255, 200}) // Semi-transparent white background
		dc.DrawRectangle(legendX, legendY, legendWidth, legendHeight)
		dc.FillPreserve() // Fill but keep path for border
		dc.SetColor(color.Gray{100})
		dc.SetLineWidth(0.5)
		dc.Stroke()

		// Draw legend items
		currentY := legendY + vPadding
		for i, name := range validColsToPlot {
			dc.SetColor(colors[i])
			dc.DrawRectangle(legendX+hPadding, currentY, boxSize, boxSize)
			dc.Fill()

			dc.SetColor(color.Black)
			// DrawStringAnchored uses the currently set font face
			dc.DrawStringAnchored(name, legendX+hPadding+boxSize+5, currentY+boxSize/2, 0, 0.5) // Align text vertically center with box
			currentY += vSpacing
		}
	}

	// Draw Title
	dc.SetColor(color.Black)
	// dc.SetFontSize(16) // <--- この行を削除またはコメントアウト
	// DrawStringAnchored uses the currently set font face (default if none loaded via LoadFontFace)
	dc.DrawStringAnchored(opts.Title, float64(w)/2, 25, 0.5, 0.5) // Centered at top

	// --- Drawing ends ---

	// Encode image to PNG in memory
	var buf bytes.Buffer
	err = png.Encode(&buf, dc.Image())
	if err != nil {
		return "", fmt.Errorf("failed to encode image to PNG: %w", err)
	}

	// Return base64 encoded string
	imgBase64 := base64.StdEncoding.EncodeToString(buf.Bytes())
	return imgBase64, nil
}

// Helper to get image from context - might be useful for debugging or direct use
func GetImage(dc *gg.Context) image.Image {
	return dc.Image()
}

// Helper to encode image.Image to PNG bytes - might be useful
func EncodePNG(img image.Image) ([]byte, error) {
	var buf bytes.Buffer
	err := png.Encode(&buf, img)
	if err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
