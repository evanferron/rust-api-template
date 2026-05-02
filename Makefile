# --- Couleurs pour l'affichage ---
HELP_COLOR=\033[36m
RESET=\033[0m

## help: Affiche cette aide
help:
	@echo "Usage:"
	@sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'

## init : Installation de rust et des composants nécessaires (clippy, rustfmt)
init:
	@echo "${HELP_COLOR}==> Installation de rust en cours...${RESET}"
	rustup update stable && rustup default stable
	cargo install --locked typst-cli
	rustup component add rustfmt
	rustup component add clippy

## build: Compile le binaire pour l'OS actuel
build: init
	@echo "${HELP_COLOR}==> Compilation en cours...${RESET}"
	RUSTFLAGS="-Ccode-model=kernel -Ccodegen-units=1" cargo build --verbose

## build-release: Compile le binaire en mode release
build-release: init
	@echo "${HELP_COLOR}==> Compilation en mode release...${RESET}"
	cargo build --release --verbose

## test: Lance les tests
test:
	cargo test --lib
	cargo test --test cucumber

CARGO_COV = cargo llvm-cov --no-report
COV_REPORT = cargo llvm-cov report --html

.PHONY: test-cov

test-cov:
	cargo llvm-cov clean --workspace
	$(CARGO_COV) --lib --bins -- --test-threads=1
	$(CARGO_COV) --test cucumber
	$(COV_REPORT)

# lint: Lint le code avec clippy et traite les warnings comme des erreurs
lint:
	@echo "${HELP_COLOR}==> Linting du code...${RESET}"
	cargo clippy --workspace -- -D warnings

# fmt: Formate le code avec rustfmt et traite les erreurs de formatage comme des erreurs
fmt:
	@echo "${HELP_COLOR}==> Formatage du code...${RESET}"
	cargo fmt --all -- --check

## start: Lance l'application simplement
run:
	@echo "${HELP_COLOR}==> Lancement de l'application...${RESET}"
	cargo run --bin server

## module: Création du module
module-gen:
	@if [ -z "$(name)" ]; then \
		echo "Erreur: Vous devez spécifier un nom. Exemple: make gen name=invoice"; \
		exit 1; \
	fi
	@echo "${HELP_COLOR}==> Création du module ${RESET}"
	cargo run --bin generate -- generate $(name)

module-del:
	@if [ -z "$(name)" ]; then \
		echo "Erreur: Vous devez spécifier un nom. Exemple: make del name=invoice"; \
		exit 1; \
	fi
	@echo "${HELP_COLOR}==> Suppression du module ${RESET}"
	cargo run --bin generate -- delete $(name)

## doc: Création de la documentation
doc:
	@echo "${HELP_COLOR}==> Generation de la documentation...${RESET}"
	cargo doc --no-deps --document-private-items --open
