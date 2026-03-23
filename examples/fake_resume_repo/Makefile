.PHONY: pdf clean

pdf: build/resume.pdf

build/resume.pdf: resume.typ
	mkdir -p build
	typst compile resume.typ build/resume.pdf

clean:
	rm -rf build
