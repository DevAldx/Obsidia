# Obsidia 0.4.2: notas de la versión

Fecha: 2026-11-01

## Cambios

Arregla la subida de packs para compartir: en la 0.4.1 el servidor no podía
confirmar la cuenta y generar un código corto fallaba siempre.

### Compartir

- **Nuevo inicio de sesión con el certificado de jugador.** Mojang bloquea las
  peticiones que llegan desde Cloudflare, así que el servidor ya no le
  pregunta. Ahora el launcher pide a Mojang el certificado de la cuenta (el
  mismo que usa Minecraft para firmar el chat) y firma con él el desafío del
  servidor. El servidor comprueba la firma de Mojang y la del launcher sin
  conectarse a Mojang.
- El token de Minecraft sigue sin enviarse nunca al servidor de Obsidia.
- Probado de punta a punta con una cuenta real: subir un pack con un mod de
  Modrinth, un mod privado, configuración e imagen, importarlo por su código
  y comprobar que todos los archivos coinciden.

### Servidor

- `server/` verifica los certificados con las claves públicas de Mojang
  incluidas en el código; si Mojang las cambia, se pueden poner otras con la
  variable `MOJANG_PLAYER_KEYS` sin tocar el código.
- Nuevas pruebas: rechaza certificados no firmados por Mojang, caducados, de
  otro UUID o firmados sobre otro desafío.

## Compatibilidad

- Importar códigos funciona igual en la 0.4.1 y la 0.4.2; solo la subida
  necesita la 0.4.2.
- Sin cambios en instancias, ajustes ni sesión.
