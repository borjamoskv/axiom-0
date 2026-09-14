# Ruta A: instantáneas atómicas con intentos acotados

Implementación: `src/seqlock.rs`. Es un componente experimental para Axiom-MM;
no define por sí solo la semántica completa de memoria del lenguaje.

## Contrato

```rust
SeqlockCell<T, const N: usize>::new(value: [T; N])
cell.try_read(attempts: usize) -> Result<Snapshot<T, N>, ReadError>
cell.try_write(value: [T; N]) -> Result<usize, WriteError>
```

`T` está limitado mediante un trait sellado a escalares con atómico nativo:
`bool`, enteros de 8/16/32/64 bits e `isize/usize`, según el soporte del destino.
`N` es el número de palabras del contenido `[T; N]`. Una estructura arbitraria
debe convertirse explícitamente a palabras fuera de la celda; no hay
reinterpretación de memoria, punteros recuperados de enteros ni callbacks.

Una instantánea correcta devuelve el contenido íntegro de la versión indicada.
Puede ser antigua: el contrato no impone frescura, edad máxima ni un límite a la
distancia entre versiones. `attempts` limita los intentos de lectura, no la
debilidad del modelo de memoria. Un error de escritura no modifica el contenido
por parte de esa llamada; otros escritores pueden modificarlo concurrentemente.

Las lecturas volátiles no son operaciones atómicas de sincronización entre
hilos; por eso el contenido utiliza atómicos reales.
[Documentación de Rust sobre `read_volatile`](https://doc.rust-lang.org/std/ptr/fn.read_volatile.html).

## Protocolo

Estado inicial: contador `0`, palabras inicializadas antes de compartir la celda
mediante los mecanismos seguros habituales de Rust.

| Operación | Orden |
|---|---|
| Lectura inicial/final del contador | Acquire |
| Adquisición del escritor: CAS fuerte `2k → 2k+1` | AcqRel; fallo Acquire |
| Escritura de cada palabra | Release |
| Lectura de cada palabra | Acquire |
| Publicación de la nueva versión `2k+2` | Release |

El lector descarta el intento si observa un contador impar. En otro caso copia
las palabras y vuelve a leer el contador. Acepta únicamente si ambos valores
coinciden. El escritor comprueba `checked_add(2)` antes de adquirir; al alcanzar
`usize::MAX - 1`, conserva la última versión legible y rechaza nuevas escrituras.

La sección del escritor solo contiene operaciones atómicas de primitivos e
iteración sobre un array fijo. No contiene código del usuario, asignaciones de
memoria dinámica ni operaciones falibles. Los campos privados impiden saltarse
el protocolo. Un escritor suspendido tras adquirir puede mantener el contador
impar; no existe recuperación automática ni espera hasta que se libere.

## Argumento de coherencia

Este argumento es una derivación del protocolo, no una certificación mediante un
asistente de pruebas. Rust describe sus atómicos con las reglas de C++20.
[Modelo atómico de Rust](https://doc.rust-lang.org/std/sync/atomic/index.html#memory-model-for-atomic-accesses).

Denotamos `sb` el orden dentro del hilo, `sw` la sincronización Release/Acquire y
`hb` su cierre transitivo. Sean:

- `B_j`: CAS que adquiere la escritura de la versión `j`.
- `D_j[i]`: escritura de su palabra `i`.
- `C_j`: publicación de su contador par `2j`.
- `A`, `L_i`, `Z`: lecturas inicial, de palabra y final del lector.

Supongamos que `A` observa `C_k` y que el intento termina aceptado (`Z = A`).

**No puede observar palabras anteriores a `k`.** Para cada palabra:

```text
D_k[i] --sb--> C_k --sw--> A --sb--> L_i
```

Por tanto, `D_k[i] hb L_i`. La coherencia escritura-lectura obliga a observar esa
escritura o una posterior. Para `k = 0`, la inicialización segura establece el
límite inferior correspondiente.
[Regla de coherencia escritura-lectura](https://eel.is/c++draft/intro.races#14).

**Tampoco puede observar palabras posteriores a `k`.** Si `L_i` toma su valor de
`D_j[i]`, con `j > k`, se obtiene:

```text
B_j --sb--> D_j[i] --sw--> L_i --sb--> Z
```

Luego `B_j hb Z`. La misma regla impide que `Z` devuelva `C_k`, anterior a `B_j`
en el orden de modificaciones del contador. Esto contradice la aceptación.
Así, todas las palabras de una lectura aceptada pertenecen a `k`. Esta segunda
cadena requiere Release/Acquire en el contenido; degradarlo a Relaxed invalida
el argumento. No hace falta una barrera adicional con estos órdenes.
[Sincronización Release/Acquire](https://eel.is/c++draft/atomics.order#2).

El CAS es una operación de lectura-modificación-escritura que lee la modificación
inmediatamente anterior del contador. Dos escritores no pueden adquirir el
mismo par. Al no reutilizar versiones, la igualdad de contadores no sufre ABA.
[Regla para RMW](https://eel.is/c++draft/atomics.order#11).

## Coste y progreso

Con presupuesto `B` y `N` palabras:

- Lectura: como máximo `B × (N + 2)` cargas atómicas; memoria temporal O(N).
- Escritura: una carga, como máximo un CAS, `N` stores y una publicación; O(N).
- Creación y almacenamiento de la celda: O(N), sin dependencias externas.
- Presupuesto cero: error inmediato, sin lecturas atómicas.
- `N = 0`: contenido vacío válido, con versiones e intentos igualmente acotados.

Son límites de operaciones del programa. No garantizan latencia física máxima,
éxito bajo contención ni progreso wait-free. Los atómicos disponibles de Rust son
lock-free, pero una operación puede contener reintentos a nivel de implementación.
[Garantías de portabilidad](https://doc.rust-lang.org/std/sync/atomic/index.html#portability).

La cota del elaborador sigue siendo la del fragmento simplemente tipado: este
módulo no introduce conversión dependiente ni normalización.

## Evidencia y alcance de las pruebas

- Pruebas unitarias fuerzan un escritor detenido a mitad de publicación y el
  último contador par. Comprueban fallo acotado, contenido intacto tras rechazo y
  ausencia de desbordamiento.
- Las pruebas de API cubren valores extremos, cada escalar soportado, arrays
  vacíos, presupuesto cero y versiones sucesivas.
- La prueba concurrente utiliza cuatro escritores y cuatro lectores. Cada
  instantánea aceptada se contrasta con el contenido registrado para su versión;
  la versión final se contrasta con el número de escrituras exitosas.
- El modelo finito de `tests/seqlock_model.rs` explora observaciones de dos
  escritores serializados, dos palabras y un lector. Incluye observaciones
  atrasadas y un control negativo con contenido Relaxed que admite mezcla.

El modelo es independiente del ejecutable: no instrumenta sus atómicos ni explora
todo Rust/C++20. Las pruebas concurrentes cubren ejecuciones reales del destino
local, no todos los comportamientos de otros procesadores. El argumento anterior
y las pruebas son evidencias complementarias con estos límites explícitos.

```sh
cargo test --offline
cargo test --offline --release --test seqlock --test seqlock_model
cargo run --offline --release --example snapshot
```
