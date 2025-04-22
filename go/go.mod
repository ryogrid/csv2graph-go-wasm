module csv2graph-wasm // Changed module name

go 1.20 // Use a compatible Go version, e.g., 1.16+ (adjust as needed)

require github.com/fogleman/gg v1.3.0

// Indirect dependencies are usually handled by `go mod tidy`
// but we include them based on your original file for completeness.
// You might need to run `go mod tidy` after creating these files.
require (
	github.com/golang/freetype v0.0.0-20170609003504-e2365dfdc4a0 // indirect
	golang.org/x/image v0.18.0 // indirect; indirect - Update if necessary via 'go get -u' or 'go mod tidy'
)
