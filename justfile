fmt:
	gofmt -w ./cmd ./internal

vet:
	go vet ./...

test:
	go test ./...

build:
	go build ./...

check: fmt vet test build
