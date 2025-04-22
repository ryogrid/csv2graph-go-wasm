package plotter

import (
	"image/color"
	"math"
)

// generateColors generates n distinct colors using HSV color space.
func generateColors(n int) []color.Color {
	var cols []color.Color
	for i := 0; i < n; i++ {
		// HSV -> RGB
		h := float64(i) / float64(n)
		s := 0.7 // Saturation
		v := 0.9 // Value (Brightness)
		r, g, b := hsv2rgb(h, s, v)
		cols = append(cols, color.RGBA{uint8(r * 255), uint8(g * 255), uint8(b * 255), 255})
	}
	return cols
}

// hsv2rgb converts HSV color values to RGB.
func hsv2rgb(h, s, v float64) (float64, float64, float64) {
	var r, g, b float64
	i := math.Floor(h * 6)
	f := h*6 - i
	p := v * (1 - s)
	q := v * (1 - f*s)
	t := v * (1 - (1-f)*s)
	switch int(i) % 6 {
	case 0:
		r, g, b = v, t, p
	case 1:
		r, g, b = q, v, p
	case 2:
		r, g, b = p, v, t
	case 3:
		r, g, b = p, q, v
	case 4:
		r, g, b = t, p, v
	case 5:
		r, g, b = v, p, q
	}
	return r, g, b
}
