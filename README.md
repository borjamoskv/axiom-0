# Micro-AXIOM-0

Andamiaje de compilador en Rust y componente experimental de instantáneas
atómicas; biblioteca y ejecutables de ejemplo, sin dependencias externas.
Usa exclusivamente `std`; prohíbe código `unsafe` propio.

```text
.
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── ast.rs
│   ├── elaborator.rs
│   └── seqlock.rs
├── docs/
│   └── MEMORY_MODEL.md
├── examples/
│   └── snapshot.rs
└── tests/
    ├── arena_safety.rs
    ├── elaboration_output.rs
    ├── quantitative.rs
    ├── reference.rs
    ├── seqlock.rs
    ├── seqlock_model.rs
    └── structural.rs
```

```sh
cargo fmt --all -- --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cargo run --offline --release
cargo run --offline --release --example snapshot
```

## Fragmento y reglas

```text
ρ ∈ {0, 1, ω}
A, B ::= Unit | A →ρ B
t, u ::= () | var(level) | λρ.t | t u | (t : A)

    + | 0  1  ω           × | 0  1  ω
   ---+--------          ---+--------
    0 | 0  1  ω           0 | 0  0  0
    1 | 1  ω  ω           1 | 0  1  ω
    ω | ω  ω  ω           ω | 0  ω  ω
```

- `0`: únicamente uso borrado; `1`: exactamente un uso; `ω`: uso libre.
- Las variables utilizan niveles absolutos de De Bruijn, indexados desde cero.
- `synthesize` infiere variables, unidad, aplicaciones y anotaciones.
- `check` comprueba lambdas contra un tipo función; los demás términos sintetizan
  y comparan su tipo canónico con el esperado.
- La aplicación combina los usos de la función con `ρ` por los del argumento.
- Las llamadas públicas parten de un contexto vacío: verifican términos cerrados.
- Cada lambda verifica su parámetro intrínsecamente, incluso dentro de un
  argumento borrado. Es una decisión conservadora de este microfragmento.

Los tipos se internan por `(ρ, dominio, codominio)`. Sus identificadores son
canónicos **dentro de una misma arena**. Cada identificador incluye un token de
procedencia: mezclar arenas devuelve un error, aunque sus índices coincidan.
Los tokens se asignan de forma atómica y no se reciclan al destruir una arena;
no son identificadores persistentes para serialización. Cada aparición de un término debe tener su propio
`ExprId`; el elaborador rechaza nodos compartidos. Los hijos se construyen antes
que el padre, lo que impide ciclos dentro de la arena.

## Cota del elaborador

Sea `N = ast.expression_count()` y `T` el número de tipos canónicos:

1. El mapa de nodos visitados y la tabla de tipos se inicializan en O(N).
2. Cada nodo alcanzable se visita una sola vez y genera como máximo cinco
   continuaciones, incluida su visita. `processed_frames()` expone ese contador.
3. Contexto, nodos y tipos usan acceso directo; igualdad de tipos en O(1).
4. Cada binder guarda los contadores de factores `0` y `ω` al entrar en su scope.
   En una variable, si aumentó el contador de ceros, su uso es `0`; si solo aumentó
   el de omegas, es `ω`; en otro caso, `1`. Así se mide la demanda desde el binder
   sin dividir por cero ni copiar vectores de usos. La demanda es local a cada
   continuación, por lo que no contamina expresiones hermanas.
5. El contexto se extiende y restaura una vez por lambda. Un único registro
   comunica el resultado entre continuaciones. No hay recursión sobre
   términos o tipos, sustitución ni normalización.

Por tanto, `synthesize/check` cuestan **O(N) tiempo y O(N) memoria auxiliar** en el
modelo RAM, con crecimiento amortizado de vectores. El interning previo cuesta
O(log T) por solicitud de constructor función; construir N términos y T tipos
distintos sin solicitudes redundantes cuesta O(N + T log T).

La cota incluye todos los nodos asignados, también los inalcanzables. Comprobar K
raíces por separado inicializa las tablas K veces: O(KN), sin caché de sesión.
Esta cota no incluye conversión de tipos dependientes.

## Salida y diagnósticos

`Elaboration` conserva el tipo raíz, el número de nodos visitados y una tabla
consultable mediante `type_of(expr)` en O(1). Solo registra nodos alcanzables y
comprobados: devuelve `None` para nodos ajenos, inalcanzables o creados después de
esa elaboración. El resultado solo se publica si termina toda la comprobación.

`Ast::push_spanned` permite adjuntar un `Span` de bytes `[inicio, fin)` a cada
nodo. `error.expression()` identifica el nodo afectado y
`error.diagnostic(&ast)` añade su rango al mensaje legible. No se expanden grafos
de tipos al formatear errores; los tipos se identifican mediante handles.
La validación de los límites UTF-8 y su correspondencia con un archivo pertenece
al futuro frontend; este núcleo conserva únicamente los rangos proporcionados.

```rust
let typed = micro_axiom_0::elaborator::check(&ast, root, expected)?;
let root_type = typed.type_of(root);
assert_eq!(root_type, Some(typed.ty));
assert!(typed.processed_frames() <= 5 * typed.visited_nodes);
```

## Verificación

- Regresiones de procedencia: lectura, construcción, elaboración, destrucción
  y creación concurrente de arenas con índices coincidentes.
- Leyes del semianillo, linealidad exacta, borrado, capturas y errores de tipos.
- Comprobador de referencia independiente con vectores explícitos de usos:
  12.000 términos reproducibles y 30.000 comparaciones de aceptación, tipo y
  categoría de error. No utiliza los contadores de demanda del núcleo.
- Metadatos por nodo, rangos de diagnóstico y límite de continuaciones.
- 30.000 lambdas anidadas en un hilo con pila de 128 KiB.

Estas pruebas aportan evidencia de corrección; no constituyen una prueba formal.
Todavía no se genera código.
Parser, universos, Π dependiente, metavariables, normalización y backend quedan
pendientes. Este andamiaje no afirma implementar ni validar esas teorías.

## Ruta A: componente experimental de memoria

[`SeqlockCell<T, N>`](src/seqlock.rs) está implementado con contenido `[T; N]`:
cada elemento se almacena en su propio atómico nativo. Un trait sellado limita
`T` a `bool`, enteros con y sin signo de 8/16/32/64 bits e `isize`/`usize`, según
el soporte atómico del destino. Cada escritura de una palabra utiliza Release
y cada lectura Acquire. No se reinterpretan objetos arbitrarios ni se usan
lecturas volátiles como mecanismo de sincronización.

`try_read(presupuesto)` devuelve una instantánea con su contenido, versión par y
número de intentos, o `ReadError::RetryBudgetExhausted`. Consume como máximo
`presupuesto × (N + 2)` cargas atómicas; un presupuesto cero falla sin leer.
`try_write([T; N])` intenta reemplazar todo el contenido una sola vez: devuelve
la versión publicada o `WriteError::Contended` si no adquiere la escritura.
Al agotarse el contador devuelve `WriteError::VersionExhausted`; las versiones
no se reutilizan por desbordamiento y la última instantánea permanece legible.
Una escritura fallida no modifica el contenido por parte de esa llamada.

`N` expresa la anchura del contenido, **no una cota de frescura**. Una lectura
aceptada contiene todas las palabras de la versión indicada, que puede ser
antigua. Los límites cuentan operaciones del programa: no garantizan latencia
máxima ni éxito bajo contención o ante un escritor suspendido.

Las pruebas del módulo fuerzan contención y agotamiento del contador;
[`tests/seqlock.rs`](tests/seqlock.rs) comprueba la API y contrasta instantáneas
con publicaciones concurrentes. Un [modelo finito independiente](tests/seqlock_model.rs)
explora observaciones de lecturas atómicas, incluidas versiones antiguas y un
control negativo con palabras Relaxed. Aporta evidencia del protocolo dentro
de su alcance; no prueba todas las ejecuciones de la implementación.
El [ejemplo de instantáneas](examples/snapshot.rs) muestra el uso de la API.

El [contrato, argumento de coherencia y límites](docs/MEMORY_MODEL.md) desarrolla
estas garantías. Este componente no implementa la teoría completa de Axiom-MM
ni depende de QTT o del elaborador. La cota del elaborador se mantiene para el
fragmento descrito arriba.
