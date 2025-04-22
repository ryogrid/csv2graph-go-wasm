// go/main.go

package main

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"syscall/js" // Import the js package

	"csv2graph-wasm/plotter" // Import the local plotter package
)

// generatePlotWasm is the function exposed to JavaScript.
func generatePlotWasm(this js.Value, args []js.Value) interface{} {
	// Basic argument validation
	if len(args) != 2 {
		return js.ValueOf(map[string]interface{}{
			"error": "Invalid number of arguments: expected 2 (csvData, optionsJSON)",
		})
	}
	if args[0].Type() != js.TypeString || args[1].Type() != js.TypeString {
		return js.ValueOf(map[string]interface{}{
			"error": "Invalid argument types: both arguments must be strings",
		})
	}

	csvData := args[0].String()
	optionsJSON := args[1].String()

	// Parse options JSON string into PlotOptions struct
	var opts plotter.PlotOptions
	// Set defaults before unmarshalling in case some fields are missing in JSON
	opts.Width = 768
	opts.Height = 512
	opts.Skip = 1
	opts.Title = "Scatter Plot from CSV"

	err := json.Unmarshal([]byte(optionsJSON), &opts)
	if err != nil {
		return js.ValueOf(map[string]interface{}{
			"error": "Failed to parse options JSON: " + err.Error(),
		})
	}

	// -- Parse "size" string if provided, overriding width/height --
	if opts.Size != "" { // Check if the Size field was provided in the JSON
		wh := strings.Split(opts.Size, "x")
		if len(wh) == 2 {
			w, errW := strconv.Atoi(wh[0])
			h, errH := strconv.Atoi(wh[1])
			// Only override if parsing is successful and values are positive
			if errW == nil && errH == nil && w > 0 && h > 0 {
				opts.Width = w
				opts.Height = h
			} else {
				fmt.Printf("Warning: Invalid format or values in 'size' option ('%s'), using defaults (%dx%d) or existing width/height.\n", opts.Size, opts.Width, opts.Height)
				// Keep existing opts.Width/Height (either default or from JSON)
			}
		} else {
			fmt.Printf("Warning: Invalid format for 'size' option ('%s'), using defaults (%dx%d) or existing width/height.\n", opts.Size, opts.Width, opts.Height)
			// Keep existing opts.Width/Height
		}
	}

	// Ensure skip is at least 1
	if opts.Skip < 1 {
		opts.Skip = 1
	}
	// Ensure width/height are positive after all parsing/defaulting
	if opts.Width <= 0 {
		opts.Width = 768
	}
	if opts.Height <= 0 {
		opts.Height = 512
	}

	// Call the plotter function
	base64Image, err := plotter.GeneratePlot(csvData, opts)
	if err != nil {
		// Return error to JavaScript
		return js.ValueOf(map[string]interface{}{
			"error": err.Error(),
		})
	}

	// Return the base64 encoded image string to JavaScript
	return js.ValueOf(map[string]interface{}{
		"base64Image": base64Image,
	})
}

func main() {
	fmt.Println("Go WASM Initialized (csv2graph)") // Log to browser console
	c := make(chan struct{}, 0)
	js.Global().Set("generatePlotGo", js.FuncOf(generatePlotWasm))
	<-c
}
