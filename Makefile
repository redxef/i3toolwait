INSTALL_BASE ?= /usr/local

install: i3toolwait install-modules
	install -Dm0755 -oroot -groot $< ${INSTALL_BASE}/bin/$<

install-modules: requirements.txt
	python3 -mpip install --upgrade --requirement $<

.PHONY: install install-modules
