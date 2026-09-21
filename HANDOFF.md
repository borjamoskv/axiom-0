# 📜 HANDOFF.md: AXIOM-0 (Micro-Núcleo QTT)

## 🎯 Objetivo
Implementación, depuración y validación isomórfica de un núcleo de Teoría Cuantitativa de Tipos (QTT) con Normalización por Evaluación (NbE), Unificación de Orden Superior (HOPU) simplificada y Argumentos Implícitos, libre de dependencias externas (`no_std` / dependencias en 0 absoluto en Cargo).

## ✅ Delta Exergético (Completado)
- **Motor NbE QTT:** Evaluación en tiempo normal (runtime) y estático con soporte para tipos dependientes (Pi, Sigma) y Universos limitados (`Type`).
- **Sistema Cuantitativo:** Rastreo estricto de usos con semiring ($0$, $1$, $\omega$) para variables borradas, lineales y sin restricciones.
- **Concurrencia Isostática (`TurbineEngine`):** Snapshot lock-free de elaboraciones (SeqLock propio) que soporta lectores infinitos sin contención en el Typechecker.
- **Unificación y Argumentos Implícitos:** Inyección estocástica de metavariables (`Expr::Hole` / `Value::Meta`) en aplicaciones de funciones implícitas.
- **Topología del Parser:** Eliminación de ambigüedades de aplicación implícita transmutando `if { }` a `if ... then ... else ...`.

## 📍 Punto Fijo $\Omega$ (Estado Actual)
- **Compilación:** `cargo check` y `cargo build` ejecutan sin advertencias ni fricción.
- **Test Suite:** 83/83 pruebas en verde (`cargo test`), incluyendo stress testing masivo (`stress_10k_rigorous.rs`, `stress_popperian_10k.rs`) y validaciones de unicidad paramétrica (`implicits.rs`).
- **Bloqueo Termodinámico:** El árbol git se encuentra sincronizado con el remoto `origin/main` en GitHub. El sistema está congelado en un estado de coherencia máxima (Cero Deuda Técnica).

## 🧠 Matriz de Gotchas (Lecciones Epistémicas)
1. **Plicity en AST y Evaluador:** La distinción entre explícito e implícito DEBE transportarse y retenerse tanto en el árbol sintáctico (`Expr::Pi`, `Expr::Lambda`, `Expr::App`) como en los valores semánticos (`Value::Pi`, `Value::Lam`, `Neutral::App`). Si se pierde en el elaborador, la instanciación de agujeros colapsa.
2. **Ciclo de Elaboración de Implícitos:** La inyección de metavariables ocurre exclusivamente en `Expr::App`, donde el tipo elaborado de la función base expone un `Value::Pi(Implicit)`. Se inyectan tantos agujeros como exija la firma antes de procesar el argumento explícito (`while let Value::Pi(Implicit...) = f_elab.ty`).
3. **Ambigüedad Gramatical ML:** Reutilizar `{}` para argumentos implícitos y para los bloques de `if/else` crea bifurcaciones impredecibles en parsers de descenso recursivo directo. La mutación a `if cond then conseq else alt` es la solución formal $O(1)$.
4. **Mutación de Archivos con Expresiones Regulares:** Las alteraciones profundas del AST usando regex frágiles provocan extinciones parciales del código. En el futuro, usar transformaciones semánticas AST a AST o sobrescrituras completas validadas.

## 🚀 Grafo de Acción (Siguiente Vector Operativo)
Secuencia de reactivación recomendada:
```bash
cd /Users/borjafernandezangulo/Downloads/AXIOM-0
cargo test
# Vector 1 sugerido: Inductive Types (W-Types)
# Archivo a modificar: src/ast.rs (Añadir Expr::Data, Expr::Constructor)
```
