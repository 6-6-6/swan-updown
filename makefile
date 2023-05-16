.PHONY: build clean install

build:
	cargo build --release

install:
	mkdir "${DESTDIR}/${prefix}/bin/" -p
	${INSTALL} target/release/swan-updown "${DESTDIR}/${prefix}/bin/"


clean:
	mkdir target -p
	rm target -rf
