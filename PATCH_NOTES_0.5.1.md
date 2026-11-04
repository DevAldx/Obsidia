# Obsidia 0.5.1: notas de la versión

Fecha: 2026-11-03

## Cambios

Arreglos para Windows: el juego vuelve a arrancar y ya no se abren ventanas de
cmd al usar el launcher.

### Arreglos

- **El juego no arrancaba en Windows** con el error «Failed to locate library:
  lwjgl.dll». El launcher descargaba las librerías nativas de Linux en vez de
  las de Windows. Ahora elige las del sistema donde se ejecuta. Las instancias
  que ya tienes se arreglan solas: la próxima vez que pulses «Jugar» se
  descargan las librerías correctas.
- **Ventanas de cmd en Windows:** al abrir el launcher aparecía una ventana de
  consola por cada Java que encontraba, y otra al lanzar el juego. Ahora Java
  se ejecuta sin ventana; el registro del juego se sigue viendo en el launcher
  igual que antes.
- Las opciones de Java que Minecraft define solo para un sistema concreto
  ahora se aplican también en Windows y macOS, no solo en Linux.

## Compatibilidad

- Sin cambios en instancias, ajustes, sesión ni códigos para compartir.
- En Linux todo funciona igual que en la 0.5.0.
