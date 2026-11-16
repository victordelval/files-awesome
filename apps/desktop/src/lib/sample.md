---
title: Manual de uso
autor: Files Awesome
---

# Escribir sin ruido

Files Awesome muestra tu Markdown **ya compuesto**, pero el fichero sigue siendo *texto plano*. Coloca el cursor en cualquier línea y la sintaxis reaparece; sal de ella y vuelve a desaparecer. Lo que guardas es, byte a byte, lo que escribiste.

## Principios

1. Nunca reescribimos lo que no has tocado.
2. Los guardados son atómicos: o el fichero viejo, o el nuevo; nunca uno a medias.
3. Si otro programa cambia el fichero, te avisamos antes de sobrescribir.

> La forma sigue a la función. Aquí, la función es leer y escribir con calma.

## Formato

Texto en **negrita**, en *cursiva*, ~~tachado~~ y `código en línea`. Un [enlace externo](https://commonmark.org) se abre con Ctrl+clic, siempre tras confirmación.

- Listas con viñeta
- Que se leen como un plano
  - Y admiten anidación

### Tareas

- [x] Núcleo en Rust con guardado atómico
- [x] Vista previa en vivo sobre CodeMirror 6
- [ ] Tablas renderizadas
- [ ] Imágenes locales

---

## Código

```rust
fn guardar(ruta: &Path, bytes: &[u8]) -> io::Result<()> {
    // temporal → fsync → rename → fsync del directorio
    atomic_write(ruta, bytes)
}
```

## Tablas

| Atajo        | Acción                  |
|--------------|-------------------------|
| Ctrl+S       | Guardar                 |
| Ctrl+P       | Paleta de comandos      |
| Ctrl+/       | Vista ↔ fuente          |
| Ctrl+.       | Modo foco               |

## Atajos de formato

Ctrl+B negrita · Ctrl+I cursiva · Ctrl+E código · Ctrl+K enlace · Ctrl+1…6 encabezados.
