# Obsidia 0.4.0: notas de la versión

Fecha: 2026-10-31

## Cambios

El launcher pasa a llamarse **Obsidia** y estrena una interfaz nueva. Las
instancias, la sesión de Microsoft, los ajustes y los códigos `NBL-` se
migran solos la primera vez que arranca (ver «Migración» más abajo).

### Nueva identidad

- Nombre, icono, splash, instalador y todos los textos pasan a Obsidia.
- Paleta «obsidiana»: grafito casi negro con un único color de acento (sigue
  siendo configurable) y esquinas talladas como firma visual.
- Fuente Sora para títulos, empaquetada en la app (unos 30 KB y sin descargas).
- Temas renombrados: Obsidiana, Basalto y Amatista.

### Interfaz

- **Barra superior propia** en lugar de la lateral: secciones, indicador de
  juego en curso, menú de cuenta y botones de ventana.
- **Inicio**: la instancia ocupa el escenario a pantalla completa. Tiene un botón
  partido para jugar o cambiar de instancia, accesos a mods y ajustes, la columna
  «Continuar» con otras instancias y las novedades de Minecraft.
- **Instancias**: lista y panel de detalle con pestañas Resumen, Mods, Ajustes y
  Registros. Los mods y los ajustes ya no se abren en ventanas emergentes.
- **Descubrir** (nueva): importar por código, inicio rápido con la última
  release o snapshot y revista de noticias y versiones.
- **Ajustes** por secciones: General, Cuenta, Apariencia (antes
  «Personalizar»), Actualizaciones, Minecraft, Java y memoria, Compartir y
  Acerca de. La barra de guardar solo aparece cuando hay cambios.
- Nuevo asistente para crear instancias, con elección visual del loader.
- Avisos emergentes para confirmaciones y errores, esqueletos de carga y
  mensajes de error integrados en cada sección.

### Funciones

- **Skin y capa** (botón «Skin» en la barra superior, menú de cuenta o Ajustes):
  vista de frente y espalda dibujada en canvas, subida de PNG con vista previa
  y detección automática del modelo slim, cambio u ocultación de las capas que
  tiene la cuenta, vuelta a la skin por defecto y biblioteca local con las
  skins subidas para volver a ponerlas con un clic. Solo para cuentas de
  Microsoft (usa la API oficial de Minecraft Services).
- **Pantalla de arranque visible**: la ventana de bienvenida nace oculta y se
  muestra cuando ya tiene contenido (antes aparecía un recuadro negro), se
  mantiene al menos 0,9 s para que se vea la marca y, si no llega a pintarse
  en 1,5 s, el launcher se abre igualmente. `index.html` incluye además una
  versión estática del splash para cualquier ventana que tarde en cargar.
- **Noticias de Minecraft**: se leen los feeds públicos del launcher oficial
  (`launchercontent.mojang.com`), se cachean 30 minutos y se guarda una copia en
  disco para mostrarlas sin conexión.
- Desde una nota de versión se puede crear una instancia con esa versión.
- Los mods se pueden buscar en su panel y se activan o desactivan con un
  interruptor.
- Al editar una instancia se avisa si su Java manual ya no existe en el PC.

### Rendimiento

- Se elimina el `backdrop-filter` de la barra lateral y del splash, y el splash
  ya no anima desenfoques.
- Las transiciones solo animan `opacity` y `transform`, y se respeta la opción
  «sin animaciones».
- Sin dependencias de interfaz nuevas (solo la fuente).

### Técnico

- Comando nuevo `fetch_news` y módulo `obsidia_core::news` con tests.
- CSP: se permiten imágenes de `launchercontent.mojang.com` y fuentes locales.
- Ventana principal sin decoración del sistema. Nueva capacidad
  `window-controls` con los permisos mínimos para arrastrar, minimizar,
  maximizar y cerrar.
- Ejecutable `obsidia`. El instalador de Linux instala en
  `~/.local/share/obsidia-launcher` y retira el AppImage y la entrada de menú
  de la versión anterior.
- Crate `obsidia-core`, paquetes `obsidia` y `obsidia-installer`, repositorio
  `DevAldx/Obsidia` y variable `OBSIDIA_BUNDLED_WEBKIT` (la antigua sigue
  funcionando).

### Migración desde 0.3.x

- Las carpetas `~/.config/nebula` y `~/.local/share/nebula` se mueven a
  `.../obsidia` al arrancar (sin copiar ni borrar) y se corrigen las rutas
  absolutas guardadas. Si no se pueden mover, se siguen usando las antiguas.
- El refresh token de Microsoft pasa del llavero `dev.nebula.launcher` a
  `dev.obsidia.launcher`: no hay que volver a iniciar sesión.
- Identificador de la app `dev.obsidia.launcher`.
- Las instancias nuevas usan el prefijo `OBS-`; las `NBL-` existentes y los
  códigos ya compartidos siguen siendo válidos.

### Pendiente de probar

- Windows: al actualizar desde 0.3.x, el instalador NSIS usa el nuevo nombre
  de producto y puede dejar instalada también la app antigua.
- Arrastrar y redimensionar la ventana sin bordes en distintos gestores de
  ventanas de Linux.
