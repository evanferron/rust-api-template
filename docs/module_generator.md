# Génère un module complet

make module-gen invoice
make module-gen blog_post   # snake_case
make module-gen BlogPost    # PascalCase accepté aussi

## Supprime un module

make module-del invoice

## Ce qui est généré automatiquement

src/db/invoice/
├── model.rs       ← struct Invoice + NewInvoice avec TODO
├── repository.rs  ← impl_base_repository! + macros + méthodes spécifiques
└── mod.rs

src/modules/invoice/
├── dto.rs         ← InvoiceResponse + Create/UpdateInvoiceRequest
├── service.rs     ← CRUD complet avec vérification user_id
├── handler.rs     ← handlers utoipa annotés
└── mod.rs         ← routes() avec middlewares

migrations/2026-03-07-123456_create_invoices/
├── up.sql         ← CREATE TABLE avec FK + index + trigger
└── down.sql       ← DROP TABLE
