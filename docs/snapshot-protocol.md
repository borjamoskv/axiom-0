# Coherencia del protocolo de instantáneas

[`SeqlockCell`](../src/seqlock.rs) devuelve, cuando la lectura tiene éxito, las
`N` palabras de una única versión publicada. La versión inicial es `0`. Esta
nota explica el argumento y el alcance del modelo ejecutable; el
[contrato general](MEMORY_MODEL.md) incluye la API y las demás pruebas.

La celda contiene atómicos nativos de un único tipo escalar admitido por el
trait sellado `AtomicValue`. Cada escritura sustituye el array completo. La
inicialización debe preceder a su publicación segura a otros hilos. Los campos
privados impiden modificar palabras o versiones saltándose el protocolo.

## Secuencia de operaciones

| Paso | Operación y orden |
|---|---|
| Lector: inicio | Lee `before` con Acquire; descarta el intento si es impar. |
| Lector: contenido | Lee cada palabra con Acquire. |
| Lector: validación | Lee `after` con Acquire; acepta solo si `before == after`. |
| Escritor: adquisición | Lee un par `v`; comprueba `v + 2` y hace un CAS fuerte `v → v + 1` con AcqRel, fallo Acquire. |
| Escritor: contenido | Escribe cada palabra con Release. |
| Escritor: publicación | Guarda `v + 2` con Release. |

El CAS exitoso lee la modificación inmediatamente anterior del contador: dos
escritores no pueden adquirir el mismo par. El siguiente escritor adquiere la
publicación del anterior, lo que ordena también sus escrituras de contenido.
La regla utilizada es la de las operaciones de lectura-modificación-escritura
de [C++20 N4861, `atomics.order` §10](https://timsong-cpp.github.io/cppwp/n4861/atomics.order#10).

La suma se comprueba **antes** del CAS. Al alcanzar `usize::MAX - 1`, se devuelve
`VersionExhausted` sin adquirir ni alterar el contenido. Ningún valor del
contador se reutiliza; así, dos lecturas iguales identifican la misma
publicación, sin ambigüedad por vuelta del contador. Un CAS fallido devuelve
`Contended` sin escrituras de contenido por parte de esa llamada.

## Por qué una lectura aceptada es coherente

Este es un argumento sobre el código, no una demostración mecanizada. Rust
documenta sus atómicos mediante las reglas de C++20, con las adaptaciones allí
descritas. [Modelo atómico de Rust](https://doc.rust-lang.org/std/sync/atomic/index.html#memory-model-for-atomic-accesses).

Llamemos `C_k` a la publicación del par `2k`, `B_k` al CAS que inicia esa
escritura y `D_k[i]` a su escritura de la palabra `i`. El lector ejecuta `A`
(contador inicial), `L_i` (palabras) y `Z` (contador final). Supongamos que acepta
`A = Z = 2k`.

Una carga Acquire que observa una escritura Release sincroniza con ella.
[C++20 N4861, `atomics.order` §2](https://timsong-cpp.github.io/cppwp/n4861/atomics.order#2).
La coherencia escritura-lectura impide leer una modificación anterior a otra
que ocurre antes de esa lectura según *happens-before*.
[C++20 N4861, `intro.races` §18](https://timsong-cpp.github.io/cppwp/n4861/intro.races#18).
Aplicadas al protocolo, estas reglas dan dos límites:

1. **No hay palabras anteriores a `k`.** `D_k[i]` precede a `C_k`; `A` adquiere
   `C_k` y precede a `L_i`. Por tanto, `D_k[i]` ocurre antes de `L_i` según
   *happens-before*, y esta lee esa escritura o una posterior. Para `k = 0`, el
   límite procede de la inicialización segura.
2. **No hay palabras posteriores a `k`.** Si `L_i` leyera `D_j[i]`, con `j > k`,
   adquiriría esa escritura Release. Como `B_j` precede a `D_j[i]` y `L_i`
   precede a `Z`, también `B_j` ocurriría antes de `Z` según *happens-before*.
   Entonces `Z` no podría leer `C_k`, anterior a `B_j` en el contador. El intento
   no podría aceptarse.

Las palabras aceptadas pertenecen, por ello, a `k`. El segundo límite necesita
Release/Acquire **en cada palabra**; mantener esos órdenes solo en el contador
no conserva este argumento. No depende de que los valores de versiones
distintas sean numéricamente diferentes.

## Presupuesto, progreso y frescura

`try_read(B)` realiza como máximo `B × (N + 2)` cargas atómicas y usa O(N)
memoria temporal. `B = 0` devuelve error sin cargas; `N = 0` admite un contenido
vacío. El campo `Snapshot.attempts` cuenta los intentos utilizados. La escritura
hace una carga y, como máximo, un CAS, `N` escrituras y una publicación.

Estas cotas cuentan operaciones del programa. Un escritor suspendido con versión
impar puede agotar todos los intentos del lector y provocar contención en otros
escritores. No hay garantía de éxito, equidad ni latencia máxima. Los atómicos de
Rust disponibles son *lock-free*, pero sus operaciones no tienen una garantía
general *wait-free*. [Portabilidad de los atómicos de Rust](https://doc.rust-lang.org/std/sync/atomic/index.html#portability).

Una lectura aceptada puede ser antigua: `N` mide palabras y `B` mide intentos;
ninguno limita su edad o la distancia a la última versión. La API no incorpora
un contrato de frescura en tiempo real.

## Qué comprueba el modelo finito

[`tests/seqlock_model.rs`](../tests/seqlock_model.rs) fija dos escritores
exitosos y serializados, dos palabras escritas en el mismo orden y un lector
con un único intento que comienza en par. Enumera `3 × 3 × 3 × 5 = 135`
combinaciones de fuentes de lectura, incluidas modificaciones antiguas. Las
palabras contienen etiquetas de generación para hacer visibles las mezclas.

El modelo construye *happens-before*, descarta ciclos y lecturas de escrituras
posteriores según esa relación, verifica cuatro reglas de coherencia por objeto
y comprueba el predecesor inmediato del CAS. Sus aserciones exigen que toda
observación permitida y aceptada coincida con su versión. También admiten
versiones antiguas coherentes y mezclas rechazadas. Como control negativo, con
contenido Relaxed admiten `before = after = 0` y palabras de generaciones
`[0, 1]`.

El modelo no importa ni ejecuta la implementación: un cambio de sus órdenes
atómicos podría dejar estas pruebas verdes. Tampoco explora historias
arbitrarias, todos los comportamientos de Rust/C++20, planificación, progreso,
frescura, desbordamiento ni generación de instrucciones. La comprobación finita
y el argumento anterior son evidencia limitada del protocolo; no constituyen
una validación formal de Axiom-MM ni de una semántica completa del lenguaje.
