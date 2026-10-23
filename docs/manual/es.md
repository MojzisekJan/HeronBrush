# Primeros pasos
Heron Brush es un editor de niveles basado en brushes. Construyes los niveles con brushes convexos, pintas y texturizas sus caras, colocas entidades, animas puertas y plataformas, y exportas el resultado a un motor de juego. También incluye assets integrados, formas paramétricas, conexiones en vivo con los editores de los motores, sesiones de edición compartidas y renderizado de imágenes fijas. Este manual viene incluido con la aplicación y funciona sin conexión.

En macOS, `Cmd` funciona en todos los lugares donde este manual dice `Ctrl`, y Opción equivale a Alt.

## La pantalla de bienvenida
Cuando Heron Brush se inicia sin un mapa, se abre la pantalla de bienvenida. A la izquierda están **Continuar** (el último mapa que tuviste abierto), **Nuevo mapa** y **Examinar** para abrir un `.map` existente, además de **Juego**, **Manual** y **Preferencias**. A la derecha hay una lista de mapas recientes con miniaturas y, a continuación, **Ejemplos**. La estrella (**Fijar arriba**) fija un mapa al principio de la lista; la papelera (**Mover a la papelera**) mueve el archivo a la papelera del sistema, de modo que se puede recuperar. Las flechas del teclado recorren la pantalla y Enter abre el elemento enfocado. Archivo → Bienvenida vuelve a mostrar la pantalla en cualquier momento.

## Tu primer mapa
1. Elige **Nuevo mapa** en la pantalla de bienvenida, o Archivo → Nuevo mapa (`Ctrl+N`).
2. Elige la herramienta Brush (`B`) en la barra de herramientas y arrastra en una vista 2D para dibujar la forma seleccionada. Comprueba su profundidad en otra vista 2D.
3. Selecciona el brush y muévelo arrastrándolo. Usa la pestaña **Cara** para darle un material o un color.
4. Guarda el mapa de origen con `Ctrl+S`. Cuando esté listo para usarlo en un juego, abre Archivo → Exportar y escribe una escena para tu motor.

## Archivos y guardado
El archivo `.map` es tu fuente editable. Guarda con frecuencia: exportar, las conexiones en vivo y el renderizado nunca guardan el mapa por ti. Archivo → Guardar como (`Ctrl+Shift+S`) añade la extensión `.map` si la omites. Archivo → Volver al mapa guardado descarta los cambios sin guardar y vuelve a cargar el archivo. Si cierras una ventana o abres otro mapa con cambios sin guardar, Heron Brush pregunta si quieres **Guardar**, **Descartar** o **Cancelar**.

Mientras un mapa guardado tenga cambios sin guardar, cada diez minutos se escriben copias de seguridad numeradas en una carpeta `autosave` junto al mapa. Se conservan las 50 copias más recientes.

Si abres un mapa mientras Heron Brush ya está en ejecución, se entrega a la ventana abierta, que pasa al primer plano. Usa `--new-instance` en la línea de comandos cuando quieras una ventana independiente (consulta Línea de comandos).

# La interfaz
La ventana principal tiene una barra de título con los menús, una barra de herramientas debajo, las vistas del mapa, el inspector a la derecha y una barra de estado en la parte inferior.

## Barra de título y menús
La barra de título muestra el logotipo, los menús, el nombre del mapa en el centro y los botones de la ventana. En macOS, los habituales botones rojo, amarillo y verde están a la izquierda. Arrastra la barra de título para mover la ventana y haz doble clic en ella para maximizarla. Mientras algo se ejecuta en segundo plano, aparece un indicador cerca del final de la barra: una conexión en vivo con un motor, una sesión en vivo o **MCP activo**. Haz clic en un indicador para detener la conexión en vivo o abrir su ventana.

- **Archivo**: Bienvenida, Nuevo mapa (`Ctrl+N`), Abrir mapa (`Ctrl+O`), Guardar (`Ctrl+S`), Guardar como (`Ctrl+Shift+S`), Volver al mapa guardado, Exportar, Renderizar imagen, Complementos y Preferencias (`Ctrl+,`).
- **Editar**: Deshacer (`Ctrl+Z`), Rehacer (`Ctrl+Shift+Z`), Cortar, Copiar, Pegar, Duplicar (`Ctrl+D`), Eliminar, Seleccionar todo (`Ctrl+A`), Repetir últimos comandos (`Ctrl+R`) y Olvidar los comandos repetibles.
- **Vista**: Vista 3D en ventana independiente, Ir a, Ajustar a la cuadrícula (`Alt+0`), Mostrar cuadrícula (`0`), Previsualizar animaciones y Mostrar caras omitidas.
- **Geometría**: Insertar texto 3D, Unir esquina, Convertir caras en patches, Sacar la selección de su entidad y Sincronizar los grupos enlazados.
- **Animación**: Animación por fotogramas clave, Previsualizar animaciones y Colocar cámara desde vista 3D (`Ctrl+Alt+0`).
- **Mapa**: Cargar archivo de puntos, Recargar el archivo de puntos y Cerrar el archivo de puntos; Cargar archivo de portales, Recargar el archivo de portales y Cerrar el archivo de portales. Muestran los rastros de fugas y los portales que escriben los compiladores de mapas.
- **Colaboración**: Sesión en vivo y Servir a un agente (MCP).
- **Ayuda**: Manual y Acerca de.

## La barra de herramientas
La barra de herramientas reúne todas las herramientas. El primer grupo es el de uso constante: Seleccionar, Brush, Corte, Vértice, Arista, Cara, Rotar y Escalar. El segundo grupo contiene Cizallar, Ensamblar, Barrido, Punto de control, Redondeo y Pintar. Pasa el puntero sobre un botón para ver su nombre y su atajo. Al hacer clic de nuevo en la herramienta activa se vuelve a Seleccionar.

Tras las herramientas vienen **Voltear horizontalmente** y **Voltear verticalmente**, y después dos interruptores: **Bloqueo** conserva el mapeado del material cuando un brush se mueve, y **UV** lo conserva cuando un brush gira (`U` lo activa o desactiva). A continuación aparecen los ajustes propios de la herramienta activa: la herramienta Brush muestra la forma y sus opciones, y la herramienta Redondeo muestra su radio, caras y esquinas con **Aplicar**. En el extremo derecho están el nombre de la herramienta, el tamaño de la cuadrícula (**Cuadrícula 16**, etc.; haz clic para elegir otro) y el menú **Ajustes de cuadrícula** con Ajustar a la cuadrícula, Mostrar cuadrícula, Previsualizar animaciones, Mostrar caras omitidas y Animación por fotogramas clave.

## Vistas del mapa
Hay cuatro vistas: la vista 3D y tres vistas planas, Superior (XY), Frontal (XZ) y Lateral (YZ). Preferencias → Vista → Disposición permite elegir Un panel, Dos paneles, Tres paneles o Cuatro paneles. Arrastra los separadores para cambiar el tamaño de los paneles. Al hacer clic en una vista, esta recibe el teclado. `Space` cambia a la siguiente vista en un panel que muestra una vista cada vez, y `Ctrl+Space` maximiza la vista que tiene el teclado y vuelve a restaurar la disposición.

Haz clic derecho sin arrastrar en cualquier vista para abrir un menú contextual con opciones para la selección y para lo que hay bajo el puntero. Ofrece agrupar, capas, grupos enlazados, crear entidades, mover brushes a una entidad o sacarlos de ella, copiar el nombre de un material y mostrarlo en el explorador de materiales.

## El inspector
El inspector de la derecha tiene cuatro pestañas. Arrastra su borde izquierdo para cambiar su anchura.

- **Mapa** (`Ctrl+1`): Capas, Grupos, Complementos, Problemas, filtros de la sección Vista y Etiquetas.
- **Entidad** (`Ctrl+2`): el explorador de entidades, las definiciones de entidades, las propiedades de la entidad seleccionada y, para botones y activadores, Activador de animación.
- **Cara** (`Ctrl+3`): UV, Materiales, Color, Barrido, Redondeo, Patch, y Retracción y bisel. Cada sección se puede plegar.
- **Assets**: la biblioteca de modelos de brushes reutilizables.

`Ctrl+5` oculta o muestra el inspector y `Ctrl+Alt+T` oculta o muestra la barra de herramientas, lo que deja más espacio a las vistas.

## La barra de estado y el panel de salida
La barra de estado de la parte inferior muestra el último mensaje del editor. Los errores aparecen en rojo. Haz clic en la barra, o pulsa `Ctrl+4`, para abrir el panel de salida con el historial completo de mensajes. La mayoría de los comandos informan aquí de lo que han hecho, por ejemplo cuántos brushes se han cortado.

## Diálogos y apariencia
Los diálogos como Preferencias, Exportar y Renderizar imagen se abren en ventanas propias. Preferencias → Vista → Esquema de colores permite elegir Seguir al sistema, Claro u Oscuro. **Opacidad de los paneles (%)** en la página Resumen hace que los paneles sean translúcidos sobre las vistas. La ventana dibuja sus propios botones con el estilo de tu sistema: nativo en macOS, estilo Windows 11 en Windows y estilo GNOME en Linux.

# Navegación y teclado
## Moverse en la vista 3D
Mantén pulsado el botón derecho del ratón y arrastra para mirar alrededor. Arrastra con el botón central para desplazar la vista. La rueda avanza y retrocede. `W`, `A`, `S` y `D` vuelan hacia delante, a la izquierda, hacia atrás y a la derecha, `Q` sube y `X` baja; mantén `Shift` para dar pasos más largos. Con **Alt + arrastrar con el botón central mueve la cámara** activado en Preferencias → Ratón, Alt con el botón central se mueve en la dirección de la vista. Las velocidades, las inversiones y el campo de visión están en Preferencias → Ratón.

## Moverse en las vistas 2D
Arrastra con el botón central para desplazar la vista y usa la rueda para hacer zoom alrededor del puntero. Al desplazar o hacer zoom en una vista plana, las demás se mantienen alineadas; **Enlazar las vistas 2D** en Preferencias → Ratón controla este comportamiento. `Ctrl+Alt+Z` restablece el zoom y `Ctrl+Shift+U` devuelve las vistas planas al origen.

`Ctrl+U` encuadra la selección en todas las vistas. Vista → Ir a (`Ctrl+Alt+M`) mueve la cámara a unas coordenadas escritas, selecciona lo que hay en una línea del archivo del mapa o mueve la selección una cantidad escrita.

## Cuadrícula y ajuste
`+` y `-` cambian el tamaño de la cuadrícula, y las teclas numéricas de `1` a `9` eligen un tamaño directamente, de 1 a 256 unidades. `0` o `#` muestran u ocultan la cuadrícula, y `Alt+0` activa o desactiva el ajuste. `S` ajusta los vértices de los brushes seleccionados a la cuadrícula actual, `Ctrl+Shift+V` los ajusta a unidades enteras y `Ctrl+Alt+Shift+V` a la cuadrícula.

## Atajos habituales
- `F3` o `Ctrl+Shift+P`: busca cualquier orden de los menús por su nombre y la ejecuta con `Enter`. `F10` abre los menús y las flechas se mueven por ellos.
- `Ctrl+S`: guardar el mapa. `Ctrl+Shift+S`: guardar como.
- `Ctrl+Z`: deshacer. `Ctrl+Shift+Z` o `Ctrl+Y`: rehacer.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: copiar, cortar y pegar. Pegar coloca los objetos en el puntero; `Ctrl+Alt+V` conserva sus coordenadas originales. Con caras seleccionadas, copiar toma su material y sus ajustes de UV.
- `Ctrl+D`: duplicar. `Ctrl+Shift+D`: duplicado enlazado de un grupo.
- `Delete` o `Backspace`: eliminar la selección.
- `Esc`: cancelar el arrastre en curso, retroceder un paso en la herramienta activa, volver a Seleccionar y, por último, vaciar la selección. `Shift+Esc` vuelve directamente a Seleccionar.
- `Ctrl+R`: repetir los últimos comandos repetibles. `Ctrl+Shift+R`: olvidarlos.
- `Ctrl+Alt+0` o `Ctrl+Alt+Num0`: colocar la cámara de render en la posición de la vista 3D.
- `F5`: volver a cargar los materiales. `F6`: volver a cargar las definiciones de entidades.

## Atajos de selección
- `Ctrl+A`: seleccionar todo, o todos los tiradores mientras está activa una herramienta de vértices, aristas o caras. `Ctrl+Shift+A`: no seleccionar nada. `Ctrl+Alt+A`: invertir la selección.
- `Ctrl+B`: seleccionar los demás objetos de la misma entidad o grupo.
- `Ctrl+T`: seleccionar todo lo que tocan los brushes seleccionados. `Ctrl+E`: seleccionar todo lo que hay dentro de ellos. `Ctrl+Shift+E`: seleccionar todo lo que hay en su columna a lo largo del eje de la vista. Estos comandos consumen los brushes con los que se selecciona.
- `Ctrl+I`: aislar la selección. `Ctrl+Alt+I`: ocultarla. `Ctrl+Shift+I`: volver a mostrarlo todo.
- `Ctrl+G`: agrupar. `Ctrl+Shift+G`: desagrupar.

## Atajos de transformación
- Las flechas y `Page Up` / `Page Down` mueven la selección un paso de cuadrícula en la dirección que indican en la vista activa. Con `Ctrl` duplican mientras mueven, y con `Alt` rotan.
- `Alt+X`, `Alt+Y`, `Alt+Z`: rotar un cuarto de vuelta alrededor de ese eje; añade `Shift` para el sentido contrario.
- `Ctrl+F`: voltear a través de la vista; `Ctrl+Alt+F`: voltear de arriba abajo. `Ctrl+Shift+X` y `Ctrl+Shift+Y` reflejan a lo largo de X o Y.
- `Insert`: colocar una entidad puntual en el centro de la selección.
- `Tab`: con un brush seleccionado, recorrer Seleccionar → Vértice → Arista → Cara → Seleccionar.

## El mapa de teclas de Blender
Preferencias → Teclado → Mapa de teclas ofrece Heron Brush (el predeterminado, usado en todo este manual) y Blender. Con Blender: `G` selecciona y mueve, `R` rota, `S` escala, `E` hace un barrido, `1`, `2` y `3` eligen las herramientas de vértices, aristas y caras, `K` corta, `B` selecciona, `A` selecciona todo y `Alt+A` anula la selección, `X` elimina, `Shift+D` duplica, `H` oculta y `Alt+H` lo muestra todo, y `Ctrl+Shift+Z` rehace. Todo lo demás es igual en ambos.

# Brushes y herramientas de modelado
Los brushes son sólidos convexos definidos por planos. Construye los objetos complejos con varios brushes sencillos en lugar de intentar que un solo brush sea cóncavo. Mantén bajo el número de brushes donde baste con una forma sencilla.

## Dibujar brushes
Elige la herramienta Brush (`B`) y arrastra en una vista. La barra de herramientas muestra **Forma a dibujar**: Ortoedro, Escalera, Arco (bóveda), Cilindro, Cono, Esferoide (UV), Esferoide (icosaedro) e Insertar texto 3D. De `Alt+1` a `Alt+7` eligen una forma directamente. Según la forma, la barra muestra **Ahuecar** y **Grosor:**, **Altura del escalón:** y **Dirección:** para las escaleras, **Eje:**, **Lados:** y el modo de círculo para las formas redondas, **Anillos:** para las esferas y **Precisión:** para el icosaedro. Configura las opciones antes de arrastrar. Comprueba el resultado en más de una vista.

## Seleccionar
Haz clic en un brush para seleccionarlo y `Ctrl`-clic para añadirlo a la selección. Un arrastre desde un espacio vacío dibuja un rectángulo de selección. En la vista 3D, `Shift`-clic selecciona una sola cara y `Ctrl+Shift`-clic añade más caras. Arrastrar un brush seleccionado lo mueve; arrastrar uno no seleccionado selecciona y mueve solo ese.

En la vista 3D, `Shift`-arrastra una cara de un brush seleccionado para extruirla. `Alt`-clic en otra cara copia en ella el material y la alineación de la cara seleccionada; `Alt+Ctrl` copia solo el material y `Alt+Shift` copia la rotación.

## Herramientas de modelado
- **Seleccionar** (`Esc`): seleccionar y mover.
- **Vértice** (`V`), **Arista** (`E`), **Cara** (`F`): arrastrar esquinas, aristas o caras. `Shift`-arrastrar desde el centro de una arista o cara añade una esquina en ese punto. `Delete` elimina las esquinas elegidas.
- **Rotar** (`R`), **Escala** (`T`), **Cizallar** (`G`): transformar la selección con tiradores.
- **Corte** (`C`): haz clic en dos o tres puntos para definir un plano de corte. `Enter` corta, `Ctrl+Enter` elige qué lado conservar, y `Backspace` o `Esc` quitan el último punto.
- **Ensamblar** (`N`): coloca puntos alrededor de un volumen y pulsa `Enter` para crear con ellos un brush convexo. `Backspace` quita el último punto.
- **Barrido** (`Y`): consulta la sección Barrido más abajo.
- **Punto de control** (`P`): remodelar la rejilla de control de un patch.
- **Redondeo** (`O`): haz clic en una arista para redondearla, o elige aristas o vértices con sus herramientas y pulsa `Enter`. Más caras dan un resultado más suave y más geometría.
- **Pintar** (`K`): consulta Materiales y UV.

## Combinar brushes
- `Ctrl+J`: fusionar los brushes seleccionados en su envolvente convexa.
- `Ctrl+K`: restar los brushes seleccionados de los brushes que tocan.
- `Ctrl+Shift+K`: ahuecar la selección, con paredes de un paso de cuadrícula de grosor.
- `Ctrl+L`: intersecar la selección.

## Barrido, redondeo, patch y bisel
**Barrido** crea geometría a lo largo de una trayectoria a partir de las caras seleccionadas. Selecciona las caras (en la herramienta Barrido, con `Alt`-clic), ajusta **Recorrido**, **Giro** (eje y ángulo), **Escala**, **Pasos** y **Trayectoria** (alineación Entero o Libre; Arco, Recta o Curva en S) en la sección Barrido de la pestaña Cara, y pulsa **Barrido**. La línea bajo los controles indica cuántos brushes se crearán.

La sección **Redondeo** redondea las aristas elegidas con **Radio**, **Caras** y **Esquinas** (A inglete o Redondeado); una sola cara produce un chaflán.

**Patch** muestra las **Columnas** y **Filas** del patch seleccionado, con botones para añadir o quitar un par. Geometría → Convertir caras en patches (`Ctrl+P`) crea patches a partir de las caras seleccionadas.

**Retracción y bisel** actúa sobre las caras seleccionadas: **Retracción** contrae la cara hacia dentro desde sus aristas y **Profundidad** la empuja hacia fuera (o hacia dentro, si es negativa). **Biselar caras** aplica ambas cosas; el tercer botón añade un panel o abre un rebaje, según el signo de Profundidad. **Unir esquina** abre un diálogo que une dos brushes en una esquina a inglete o redondeada, con vista previa.

## Texto 3D
Geometría → Insertar texto 3D (también la última entrada de la lista de formas de la herramienta Brush) crea brushes a partir de texto. Usa Heron Block, una fuente poligonal integrada con esquinas recortadas, anchos proporcionales y acentos checos; las minúsculas usan las formas de las mayúsculas. Escribe el **Texto**, ajusta **Altura** (la altura de las mayúsculas; los acentos sobresalen por encima) y **Profundidad**, observa la vista previa en directo y pulsa **Insertar**.

Si una operación no puede producir geometría válida, ajusta sus dimensiones, o deshazla y simplifica la forma.

## Matrices
Geometría → Matriz copia la selección y muestra las copias en el mapa mientras cambias los valores. Las cantidades incluyen el original y una matriz crea como máximo 1024 copias.

- **Fila** desplaza cada copia respecto a la anterior en **Paso X**, **Paso Y** y **Paso Z** y la gira **Giro por copia (grados)** alrededor del eje elegido.
- **Cuadrícula** llena **Cantidad X**, **Cantidad Y** y **Cantidad Z** elementos separados por **Paso X**, **Paso Y** y **Paso Z**.
- **Anillo** reparte los elementos sobre **Ángulo (grados)** alrededor de un centro a **Radio (unidades)** de la selección. Con **Orientar hacia el centro** cada copia gira con el anillo.
- **Dispersión** coloca las copias en lugares aleatorios dentro del radio, a través del eje elegido. La misma **Semilla** da la misma disposición, y **Giro aleatorio** gira cada copia.

Cuando la selección es un grupo entero, **Copias vinculadas** convierte cada copia en un duplicado vinculado, así que un cambio en una llega a todas. **Aplicar** conserva las copias y las selecciona junto con el original; **Cancelar** las elimina. Toda la matriz se deshace en un solo paso.

## Esculpir terreno
El terreno se compone de muchos brushes pequeños cuya superficie el pincel de esculpir empuja hacia fuera y hacia dentro. Selecciona uno o más brushes y elige Geometría → Dividir en columnas y luego la forma de las columnas: Triángulo, Cuadrado o Hexágono. Los corta a lo largo de las líneas de la cuadrícula en columnas de un paso de cuadrícula de ancho y pone las columnas en un grupo llamado Terrain. Los cortes siguen la cuadrícula del mundo, así que los brushes contiguos se cortan en los mismos sitios y comparten sus esquinas. Cada columna se alza a lo largo del lado delgado del brush: en vertical en un suelo o un techo, en horizontal en una pared. Una cueva se hace igual: vacía una caja (Hollow selection) con un grosor de pared de un paso de cuadrícula, selecciona las paredes y divídelas.

La ventana **Esculpir**, que abre el botón al final de la barra de esculpir, también puede dividir la selección, en la forma elegida en su fila Forma de las columnas. Los dos sitios recuerdan la última forma usada. **Triángulo** crea dos por celda de la cuadrícula, y la parte superior de un triángulo sigue plana por mucho que la esculpas. **Cuadrado** crea una por celda, la mitad de brushes; su parte superior se dobla a lo largo de una diagonal. **Hexágono** cubre el brush con hexágonos de dos pasos de cuadrícula de ancho, cada uno dividido en seis triángulos alrededor de su centro, así que cada parte superior sigue plana.

Elige la herramienta **Esculpir** (`J`) y arrastra sobre la superficie en la vista 3D. Si mantienes el botón pulsado sin mover el ratón, el pincel sigue trabajando el mismo punto, diez veces por segundo, así que una colina crece mientras lo mantengas. El anillo naranja muestra el pincel. Sin selección, el pincel cambia todos los brushes que tiene debajo; con selección, solo los seleccionados. La barra de la parte superior de la ventana elige qué hace un trazo:

- **Subir** y **Bajar** mueven la superficie hacia fuera o hacia dentro, más en el centro del anillo y nada en el borde. Mantén `Shift` para intercambiarlos.
- **Suavizar** iguala los bultos.
- **Aplanar** nivela la superficie hasta donde empezó el trazo.

En la misma ventana, **Dirección** elige hacia dónde:

- **Arriba** mueve la superficie en vertical, arriba y abajo, dentro de un círculo sobre el suelo. Es para terreno.
- **Cámara** la acerca y la aleja de la vista, dentro de una esfera alrededor del puntero. Mira un suelo, una pared o un techo y el pincel le da forma.
- **Normal** la saca de la cara que está bajo el puntero donde empieza el trazo.

Los dos números son **Radio (unidades)** y **Fuerza (%)**. Solo se mueven las esquinas del lado que mueve el trazo; la parte de atrás se queda donde está y una columna mantiene al menos una unidad de grosor. Donde los brushes comparten una esquina, la mueven juntos, así que la superficie sigue cerrada. Un trazo se deshace en un solo paso. En las vistas 2D la herramienta selecciona como la herramienta Seleccionar.

Cuando termines de esculpir, selecciona el terreno y elige Geometría → **Unir columnas**, o el botón de la ventana **Esculpir**. Las columnas vecinas que juntas forman un brush convexo y se ven igual pasan a ser un solo brush, así que las zonas planas quedan en unos pocos brushes grandes. La superficie no se mueve y la unión se deshace en un paso.

**Fusionar conservando la forma**, justo al lado en el menú y en la ventana, une de la misma manera pero no se fija en el aspecto de las caras: donde dos caras pasan a ser una, esta lleva el material de una de ellas. Funciona con cualquier brush seleccionado, no solo con terreno. Cuatro cajas en L quedan en dos, nunca en una que rellene la L, y todo un terreno esculpido se reduce a tan pocos brushes como encuentre la unión de vecinos. Solo se unen brushes del mismo grupo o entidad, así que una puerta nunca se traga la pared de al lado.

Para un terreno como el de los juegos de vóxeles, divide en columnas **Cuadrado** y elige **Bloques** en **Trazo** dentro de la ventana **Esculpir**. Un trazo mueve entonces toda la parte superior de cada columna por pasos enteros de cuadrícula y deja sus lados verticales, así que el suelo sube en escalones. Todos los modos funcionan igual; los toques pequeños se suman hasta un paso entero. Las columnas de otras formas no cambian.

**Libre** no necesita columnas: funciona con cualquier brush. Subir añade terreno en una bola alrededor del puntero y Bajar lo quita, así que el suelo puede formar salientes y cuevas. Suavizar lo iguala y Aplanar lo lleva al nivel donde empezó el trazo. Cada toque reconstruye en la cuadrícula los brushes que alcanza, redondeados, y deja todo lo que queda fuera de su bola como estaba, aristas vivas incluidas. El resultado son brushes normales, así que cualquier otra herramienta funciona con ellos, y **Unir columnas** reduce su número después. Una cuadrícula más fina da una superficie más suave y más brushes.

# Materiales y UV
## Cargar y aplicar materiales
Abre la sección **Materiales** de la pestaña Cara. **Cargar carpeta** añade una carpeta de imágenes y **Cargar .wad** añade un paquete de texturas; ambos aparecen en **Colecciones**, donde la cruz quita uno. Escribe en **Filtrar materiales** para acotar la lista. Haz clic en un material para que sea el actual y, después, **Aplicar a la selección** lo pone en todas las caras seleccionadas. **Seleccionar** ofrece Seleccionar los brushes que lo usan y Seleccionar las caras que lo usan. **Reemplazar** cambia un material por otro en todo el mapa o solo en la selección.

La herramienta **Pintar** (`K`) pone el material actual en la cara bajo el puntero: un clic pinta una cara, un doble clic pinta todo el brush y `Shift`-clic toma el material de la cara como material actual, como un cuentagotas.

## El editor de UV
La sección **UV** de la pestaña Cara muestra el material y el contorno de la cara seleccionada. Controles del ratón en el lienzo:
- La rueda hace zoom alrededor del puntero. Arrastra con el botón derecho o el central para desplazar la vista. Un doble clic con el botón derecho vuelve a encuadrar la cara; al seleccionar otra cara, se encuadra automáticamente.
- Arrastrar con el botón izquierdo mueve el material mientras el contorno de la cara se queda en su sitio. El anillo rota el mapeado, los tiradores de las esquinas lo escalan y el tirador central fija el origen de estos cambios.
- `Alt`-arrastrar con el botón izquierdo sobre el borde de una tesela cizalla, igual que `Shift`-arrastrar con el botón central.
- Cada arrastre es un paso de deshacer, y `Esc` cancela un arrastre en curso.

Bajo el lienzo están **Desplazamiento**, **Escala**, **Ángulo** y, cuando el juego los usa, **Indicadores de contenido**, **Indicadores de superficie** y **Valor de superficie**. Los botones son **Restablecer**, **Restablecer según el mundo**, **Encajar**, **Voltear H**, **Voltear V** y los dos giros de 90°. **Alinear** gira el material a lo largo de una de las aristas de la cara (pulsa de nuevo para la arista siguiente, con `Shift` para la anterior). Los cuatro botones de justificación empujan el material contra un borde. **Ajustar U** y **Ajustar V** escalan para que un número entero de copias cubra la cara; pulsa de nuevo para el siguiente número, y mantén `Ctrl` para conservar la subdivisión de una trim sheet.

## Teclas de UV
Con caras seleccionadas, las flechas mueven el material, `[` y `]` lo giran 15°, `,` y `.` reducen a la mitad o duplican su escala, `Ctrl+F` y `Ctrl+Alt+F` lo voltean, `Shift+R` lo restablece, `Alt+Shift+R` lo restablece según los ejes del mundo y `Ctrl+Shift+F` lo ajusta a cada cara.

**Bloqueo** en la barra de herramientas conserva el mapeado mientras los brushes se mueven, y **UV** (`U`) lo conserva mientras los brushes giran.

## Materiales de herramientas
- skip: quita la cara marcada de las exportaciones a motores.
- clip: crea colisión sin geometría visible.
- origin: marca el pivote de una entidad de brush.

Los nombres se pueden cambiar en la tarjeta **Materiales de herramientas** de la ventana de exportación. Usa skip para las caras que sabes que quedan ocultas.

## Omitir caras en la exportación
Selecciona una cara y activa **Omitir cara al exportar** en la sección UV. Un rayado naranja la marca en la vista previa de UV. Su material y su pintura siguen guardados, y el brush sigue cerrado y editable. Las mallas de render exportadas dejan fuera esa cara, pero se conserva la colisión. Si se deja un solo quad de un cubo, se obtienen dos triángulos de render. Esto no hace que las caras sean de doble cara ni fusiona mallas.

**Omitir caras cubiertas** marca las caras totalmente cubiertas por brushes estáticos, incluidas las caras idénticas que se tocan y los brushes encerrados. Actúa sobre los brushes seleccionados o, si no hay nada seleccionado, sobre toda la geometría del mundo visible y desbloqueada. Se omiten las entidades de brush, los materiales transparentes y los brushes que ya tienen caras omitidas. Las caras cubiertas solo en parte no se dividen. Un solo deshacer lo restaura todo. Vista → Mostrar caras omitidas oculta o muestra esas caras en las vistas del editor; la selección sigue usando el brush completo.

# Color y degradados
La sección **Color** de la pestaña Cara pinta las caras con colores planos o degradados por encima de sus materiales. Es útil para el blockout de un nivel y para el arte estilizado. Las caras pintadas conservan sus colores reales en las vistas; la selección se indica con su contorno.

## Paleta y color de mezcla
Haz clic en una de las doce muestras de la paleta para pintar con ella las caras seleccionadas, o los brushes seleccionados completos. **Opacidad** fija la opacidad de la siguiente pintura, en porcentaje. **Mezclar** tiene un botón de color que abre el selector de color; arrastra en el selector y la selección lo sigue en directo, o pulsa **Pintar** para aplicar el color de mezcla en un solo paso. **Volver al material** quita la pintura para que la cara vuelva a mostrar su material. La línea de la parte superior de la sección indica qué está seleccionado y qué lleva.

## El selector de color
Todos los botones de color del editor abren el mismo selector. Arrastra en el cuadrado para elegir la saturación y el brillo, y en la franja de al lado para elegir el tono. Debajo están los valores **R**, **G** y **B** y un control deslizante de **Opacidad** cuando el color lo admite. El campo hexadecimal acepta `#RRGGBB` o `#RRGGBBAA`; se pone rojo mientras el texto no sea un color válido. **Copiar** copia el código hexadecimal al portapapeles. Haz clic fuera del selector para cerrarlo.

## Degradados
**Degradado** elige Degradado lineal o Degradado radial, y el botón de al lado invierte los colores de un extremo a otro. Cada punto tiene un botón de color y **Posición (%)**, y los puntos intermedios tienen un botón para quitarlos. **Añadir punto de color** añade un punto en el hueco más amplio; un degradado puede tener hasta ocho puntos. **Ángulo del degradado** fija la dirección de un degradado lineal, con los botones **+45°** y **−45°**.

Bajo la vista previa hay un riel con un marcador para cada punto. Haz clic en el riel para añadir un punto, haz clic en un marcador para abrir su selector, arrastra un marcador para moverlo y haz clic derecho en un marcador para quitarlo. Las tres muestras bajo el riel son ajustes predefinidos: Degradado cálido, Degradado frío y Degradado de grises. Los cambios en un degradado se aplican a la selección en directo.

La dirección de un degradado importa: oriéntalo a lo largo de la pendiente de un tejado o de un objeto curvo, no a través de caras que no tienen relación. Después de pintar, revisa el modelo por ambos lados.

## Material de superficie
**Material de superficie** ajusta **Rugosidad** (0 es brillante, 100 es mate), **Metalicidad** (0 para madera, piedra o plástico, 100 para metal) y **Emisión** (de 0 a 8; la superficie brilla con su propio color, degradados incluidos, pero no ilumina los objetos cercanos). Los cambios se aplican en directo a las caras o brushes seleccionados. **Usar superficie predeterminada** quita el ajuste propio. Todos los cambios se pueden deshacer.

Los efectos de superficie se ven en la vista 3D iluminada y en los renders de cámara. Las visualizaciones plana y de malla de alambre omiten los brillos a propósito. El bloom puede añadir un resplandor alrededor de las superficies emisivas. Godot y glTF/GLB reciben los valores del material, y el importador de Unity convierte la rugosidad en suavidad (smoothness). Consulta `docs/surface-materials.md` para conocer los límites de cada destino.

# Assets, capas y grupos
## La biblioteca de assets
Abre la pestaña **Assets**. Añade una carpeta de modelos `.map` con **Añadir carpeta**; la biblioteca integrada siempre está disponible. Escribe en **Filtrar assets** para buscar. Haz clic en un modelo para seleccionarlo y, después, haz doble clic en él, pulsa `Enter` o haz clic en **Insertar** para añadirlo donde estuvo el puntero por última vez. También puedes arrastrar un modelo a una vista. Los brushes de un asset se insertan como un solo grupo para que el modelo se mueva como un todo.

**Guardar la selección como asset** escribe los brushes seleccionados en la biblioteca como un `.map`. Los assets de la biblioteca son brushes normales y editables, no mallas importadas.

## Capas
La lista **Capas** de la pestaña Mapa organiza las zonas grandes. Escribe un nombre y haz clic en **Añadir** para crear una capa, o pulsa `Enter` para renombrar la actual. Cada capa se puede ocultar, bloquear o excluir de las exportaciones. **Mover aquí** mueve la selección a la capa actual. El menú contextual de las vistas también ofrece Mover a la capa, Hacer capa activa, Ocultar capas y Aislar capas.

## Grupos y grupos enlazados
`Ctrl+G` agrupa la selección y `Ctrl+Shift+G` la desagrupa. La sección **Grupos** de la pestaña Mapa enumera los grupos y ofrece **Abrir**, **Renombrar**, **Fusionar**, **Desagrupar**, **Sacar** y **Extraer**. Un grupo enlazado es un conjunto de copias que comparten las ediciones: `Ctrl+Shift+D` crea un duplicado enlazado, Geometría → Sincronizar los grupos enlazados actualiza las copias y **Desenlazar** hace que una copia sea independiente. En la pestaña Entidad, una propiedad puede mantenerse como propia de una copia cuando se sincronizan los grupos enlazados.

## Una escala común
La escala de exportación predeterminada es de 32 unidades de mapa por metro. El marcador de inicio del jugador integrado mide 64 unidades de alto, una referencia de 2 metros. Compara con él puertas, vehículos, muebles y plantas. Cambia la escala de exportación solo a propósito y mantén coherentes los assets relacionados.

# Entidades y jugabilidad
Las definiciones de entidades más comunes vienen integradas y están siempre disponibles: luces, inicios del jugador, puertas, botones, plataformas, activadores, props estáticos, sonidos y waypoints. Cargar el FGD de un proyecto con **Definiciones de entidades** en la pestaña Entidad sustituye las clases con el mismo nombre y conserva el resto de las integradas.

## Colocar entidades
En el explorador **Entidades** de la pestaña Entidad, escribe en **Filtrar clases** y elige una clase. Para una clase puntual, como una luz, haz clic en **Colocar** o haz doble clic en la clase; `Insert` coloca una en el centro de la selección. Para una clase de brush, como una puerta o un activador, selecciona los brushes y haz clic en **Generar**. Las clases también se pueden arrastrar a una vista.

## Propiedades
La lista de propiedades edita la entidad seleccionada. Las entidades de la misma clase se editan juntas. **Añadir propiedad** añade una clave que la definición puede no conocer; **Valores predeterminados** puede devolver las propiedades existentes a sus valores predeterminados o añadir las que faltan. Las propiedades que la clase define pero que la entidad no tiene se muestran atenuadas; al escribir un valor se añaden. Usa targetname y target para enlazar entidades.

## Qué puede hacer una exportación
Las directivas de Godot crean nodos nativos, cuerpos de colisión y áreas de activación. Unity y Unreal admiten de forma nativa las luces habituales, y Unreal además crea actores PlayerStart. El resto de los datos se conserva en la medida en que lo permite el importador del destino.

Una entidad de puerta por sí sola no añade comportamiento de juego. El movimiento, las respuestas a los activadores, los controladores del jugador y los scripts del proyecto siguen necesitando trabajo en el motor. Defold exporta las propiedades de las entidades y los objetos de colisión; Flax coloca modelos y entidades genéricas. Comprueba el resultado en tu motor antes de construir la jugabilidad sobre él.

# Animación
Heron Brush puede previsualizar movimientos sencillos y editar animaciones por fotogramas clave de entidades de brush, y luego exportarlas a varios motores.

## Previsualizar animaciones
Activa **Previsualizar animaciones** en el menú Vista, el menú Animación, el menú Ajustes de cuadrícula o la sección Vista de la pestaña Mapa. Reproduce la rotación de func_rotate y el vaivén de func_shuttle cuando start_on está activado, usando las propiedades de eje, velocidad, dirección, recorrido y espera de la entidad, y reproduce los clips de fotogramas clave configurados con Reproducir al iniciar. Los assets del ventilador y del molino ya contienen piezas que giran.

Al desactivar la previsualización se restaura la pose original. Un clic en una vista detiene la reproducción antes de seleccionar o editar; la navegación de la cámara sigue funcionando. La previsualización no modifica el mapa, el historial de deshacer ni las exportaciones, y no ejecuta física ni lógica de juego.

## El editor de fotogramas clave
Ábrelo con Animación → Animación por fotogramas clave o desde el menú Ajustes de cuadrícula. Se acopla bajo las vistas, o en la ventana 3D mientras esta está separada. El icono de ventana de su encabezado lo pasa a una ventana propia; conserva el fotograma y el historial de deshacer. **Editar animación** cambia las vistas a la pose animada y **Editar mapa** vuelve a la geometría en reposo. En el modo de animación, cada vista muestra **Editar animación** y el número de fotograma en la parte inferior.

Selecciona una entidad de brush, o un brush o un grupo de brushes, para animarlo. Un grupo plano de brushes comparte una animación y un pivote; los grupos que contienen varias entidades necesitan animaciones separadas.

## Clips
El selector **Animación** cambia entre clips con nombre. Escribe un nombre en **Nombre de animación** y haz clic en **Guardar como** para copiar el clip actual con ese nombre; **Eliminar** quita un clip. **Trasladar** crea un clip `translate` sin rotación. **Animaciones de puerta** crea clips suaves `open` y `close` con un giro de 90 grados alrededor del pivote actual; antes, mueve el pivote a la bisagra. **Reproducir al iniciar** hace que el clip se reproduzca cuando empieza el nivel; el ajuste predefinido de puerta lo desactiva.

## Transporte y reproducción
Los botones de transporte son **Inicio**, **Clave anterior**, **Reproducción**, **Clave siguiente** y **Final**. **Fotograma** muestra y fija el fotograma actual, y **Duración (s)** fija la duración del clip. El modo es **Una vez**, **Bucle** o **Ida y vuelta**.

## La línea de tiempo
La línea de tiempo tiene una regla con el cabezal de reproducción y una pista para **Posición** (X, Y y Z se registran juntas) y para cada ángulo de rotación, RX, RY y RZ. Haz clic o arrastra en la regla para mover el cabezal y previsualizar esa pose en todas las vistas. Arrastra con el botón central para desplazar la línea de tiempo. Arrastra las claves para moverlas en el tiempo; `Shift` añade a la selección; haz doble clic en una pista para insertar una clave; `Ctrl`+rueda hace zoom. **Curvas** muestra las curvas de valores en lugar de las pistas.

Los botones de encima son **Añadir clave**, **Quitar**, **Copiar**, **Pegar** y **Duplicar**. **Ajustar a fotogramas** mantiene las claves en fotogramas enteros y **Encajar** muestra el clip completo. **Ajustar tiempos** abre **Escala temporal**, **Desplazamiento (fotogramas)**, **Escalar desde el cursor**, **Ampliar duración** y **Ajustar tiempos de selección**.

El inspector de claves, junto a la línea de tiempo, edita la clave seleccionada: **Fotograma**, **Valor** (o las tres coordenadas de una clave de Posición), la interpolación **Lineal**, **Constante** o **Bézier**, **Pendiente de entrada** y **Pendiente de salida**, y **Suavizar tangentes**.

## Teclas en el editor
Mientras la línea de tiempo tiene el teclado: `Space` reproduce o pausa, las flechas izquierda y derecha avanzan o retroceden un fotograma, `Ctrl+A` selecciona todas las claves, `Ctrl+C`, `Ctrl+X` y `Ctrl+V` copian, cortan y pegan claves, `Ctrl+D` las duplica, `Delete` las quita, `Ctrl+Z` deshace y `Esc` cancela un arrastre. Fuera de la línea de tiempo, `Esc` sale del modo de animación. Deshacer deja el zoom, la selección y el cabezal donde estaban.

## Posar en las vistas
La fila bajo la línea de tiempo contiene **Papel cebolla** con un número y **Separación**, que muestra las poses cercanas como fantasmas; las coordenadas de **Pivote**; **Claves automáticas**; y las herramientas de pose **Trasladar**, **Rotar** y **Pivote**.

- **Añadir clave** activa Claves automáticas y pausa la reproducción. Con Claves automáticas activado y **Trasladar** elegido, arrastra un eje de color del tirador de la entidad seleccionada en la vista 3D para registrar su posición en el fotograma actual.
- En una vista 2D, arrastra el brush seleccionado o un miembro de su grupo para registrar la traslación en los dos ejes visibles, con ajuste a la cuadrícula. Para esto no hace falta tener activado Claves automáticas.
- Con **Rotar**, arrastra el brush en una vista 2D alrededor de su marcador de pivote naranja. Superior registra RZ, Frontal registra RY y Lateral registra RX, ajustados a los ángulos de la cuadrícula.
- Con **Pivote**, arrastra el centro de rotación amarillo en cualquier vista 2D. Se ajusta a la cuadrícula y mantiene su profundidad. Las claves y la geometría en reposo no cambian.

La trayectoria cian en 2D y 3D muestra el recorrido del pivote. Haz clic en un punto de la trayectoria para seleccionar su tiempo y sus claves, o arrástralo para cambiar la posición; con Claves automáticas activado también puedes arrastrar puntos de la trayectoria en la vista 3D. Las etiquetas muestran los números de fotograma y los tramos discontinuos marcan los saltos. `Esc` cancela cualquiera de estos arrastres, y un solo deshacer revierte un arrastre completo.

Las animaciones nuevas empiezan en reposo con claves en el tiempo cero. Crea el movimiento cambiando la pose en otro fotograma. Para una trayectoria en L con una esquina marcada, registra el inicio, la esquina y el final, y usa Lineal con Ida y vuelta.

## Conectar botones a animaciones
Selecciona un func_button, trigger_multiple o trigger_once. La pestaña Entidad muestra **Activador de animación** encima de las propiedades. Elige un destino de la lista, o haz clic en **Elegir destino en la vista** y luego en un objeto animado de cualquier vista; `Esc` cancela la elección. Los destinos o clips que faltan se indican bajo los controles. Los enlaces entre entidades muestran flechas hacia sus destinos cuando la visualización de enlaces está activada.

**Acción** elige qué hace la activación: **Reproducir** inicia un clip con nombre o la animación seleccionada; **Alternar** alterna entre `open` y `close` cuando existen y, si no, reinicia la animación seleccionada; **Abrir** y **Cerrar** inician esos clips; **Pausar** mantiene la pose, **Reanudar** continúa y **Detener** restaura la pose original. **Solo una vez** anula el valor predeterminado de la clase. **Intervalo de repetición (s)** fija el tiempo mínimo entre activaciones; no retrasa la primera.

**Probar activador** ejecuta la acción en la previsualización sin editar el mapa ni añadir pasos de deshacer. **Restablecer vista previa** restaura las poses originales y borra el estado de una sola vez y de espera. Esto no simula las colisiones del jugador ni los scripts del juego.

## La animación en las exportaciones
Godot exporta recursos nativos AnimationPlayer y AnimationLibrary, incluidas las curvas y la colisión en movimiento. Un CharacterBody3D que entra en el área de un botón activa su destino. Las ediciones hechas en estos clips en Godot no vuelven al mapa y se sustituyen en la siguiente exportación.

La exportación a raylib añade `<map>_animation.h` con un reproductor en C y `<map>_preview.c` con una vista previa de cámara libre; sigue `<map>_raylib.md` para compilarla. En la vista previa, `SPACE` pausa, `R` reinicia los clips predeterminados y `N` recorre los clips con nombre. Defold integra la reproducción en el script de la entidad con mensajes `heron_*_animation` documentados. Build Level de Unity añade `HeronBrushAnimation` y un cuerpo cinemático. La exportación a Bevy incluye `heron_animation.rs` e instrucciones. Godot, Unity, Defold, Bevy y raylib conservan los enlaces de los activadores, el indicador de una sola vez y el intervalo de repetición. Tu juego sigue teniendo que registrar al jugador y conectar sus propios botones a la API de cada runtime.

## El ejemplo Motion lab
Abre `motion_lab` desde los Ejemplos de la pantalla de bienvenida. Contiene una plataforma con trayectoria en L, un ascensor, una plataforma giratoria y una puerta con bisagra y botones. Abre en Godot el `godot/project.godot` que hay junto a él y pulsa `F5`. WASD y el ratón mueven al jugador, Espacio salta, del 1 al 4 llevan a las estaciones y R reinicia la estación actual. Pisa una placa de color para activar su destino. El README incluido explica cómo volver a exportar.

# Exportar a un motor
Archivo → Exportar abre la ventana de exportación. Los motores aparecen a la izquierda con sus logotipos: Godot, Defold, three.js, Babylon.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb y, por último, `.map`. La página de la derecha indica qué recibirá el destino. **Escribir** lo escribe, **Comprobar** construye el nivel sin escribirlo y enumera los problemas, y **Cerrar** cierra la ventana.

Exportar es independiente de guardar: el `.map` sigue siendo tu fuente editable. Cada control guarda los ajustes de su motor en cuanto cambia, así que cada motor conserva sus propias respuestas y la ventana se abre tal como la dejaste.

## Opciones comunes
- **Escala**: **Unidades por metro**, 32 por defecto.
- **Qué escribir**: **Geometría de colisión**.
- **Optimizaciones**: **Agrupar mallas estáticas en celdas de 1024 unidades**, **Descartar las caras enterradas dentro de la geometría**, **Unir las caras que comparten plano** y **Unir las formas de colisión**.
- **Rutas del proyecto**: dónde van el nivel y sus recursos dentro del proyecto.
- **Materiales de herramientas**: los nombres de los materiales clip, skip y origin.
- **Más**: opciones específicas de cada motor, como **Nombrar nodos según**, **Reconstruir capas y grupos como nodos padre**, **Invertir el orden de vértices de los triángulos**, la inversión de las coordenadas de textura y un segundo conjunto de UV para la iluminación precalculada con **Téxel del mapa de luz**.
- **Mantener al día con el mapa**: **Volver a escribir cada vez que se guarde el mapa**. Está disponible después de haber exportado una vez para ese motor, porque esa exportación indica dónde escribir.

Las celdas estáticas asignan los brushes del mundo según su centro y los combinan por material dentro de cada celda de 1024 unidades, para que los motores puedan descartar (culling) partes más pequeñas del mapa. La colisión se conserva y las entidades de juego quedan separadas. Puede suponer más llamadas de dibujo que una sola malla del mundo, por lo que está desactivado por defecto. Las exportaciones independientes a OBJ, glTF y GLB no usan celdas.

## Notas sobre los destinos
- Godot: rutas `res://` de texturas y materiales, la capa física de los activadores y UV2 para lightmaps. Elige una carpeta de exportación dentro del proyecto de Godot.
- Defold: el directorio de recursos de assets, el material del modelo y el sampler de texturas, que debe existir en el material elegido.
- Unity: mallas OBJ y un importador para el editor; exporta dentro de la carpeta Assets del proyecto.
- Unreal Engine: un script de importación generado y una ruta de contenido, normalmente /Game/<level>.
- Flax Engine: una ruta de contenido para el modelo, relativa a la carpeta Content del proyecto.
- three.js, Babylon.js, Bevy, raylib y Cocos Creator: escenas o código para esos runtimes, con instrucciones junto a la exportación.
- glTF / GLB: geometría portable para herramientas compatibles; GLB es la variante binaria. OBJ: geometría y materiales para herramientas de modelado.
- `.map`: **Escribir como** elige Valve 220, Estándar, Quake 2 o Quake 2 (Valve). La ventana indica qué se perdería con el dialecto elegido.

Lee los avisos después de escribir, como las texturas que faltan. **Resultados de la comprobación** los enumera en la ventana.

# Conexiones en vivo: Godot, Unity, Defold y Flax
Una conexión en vivo envía las ediciones sin guardar de Heron Brush al editor de un motor mientras trabajas. Es unidireccional: las ediciones hechas en el motor no vuelven. Puede haber una conexión en vivo por cada ventana de mapa. Detenla antes de cambiar de motor y reiníciala para aplicar las opciones de exportación modificadas. Los controles están en la ventana de exportación, en la página del motor, en la tarjeta **Conexión en vivo con**.

El indicador de la barra de título muestra el estado de la conexión; haz clic en él para detenerla. Al cerrar o sustituir el mapa se deja de publicar. Tras un fallo de importación, corrige el problema y vuelve a iniciar la conexión.

## Godot
1. Exporta una vez a un proyecto de Godot.
2. Haz clic en **Instalar complemento de Godot**.
3. Activa Heron Brush Live en Configuración del proyecto → Plugins de Godot.
4. Haz clic en **Iniciar conexión en vivo** y vincula el mapa desde el panel de Godot a una escena 3D abierta.

El complemento sustituye solo su rama generada y conserva la colocación del punto de vinculación y los demás nodos. Guarda tú mismo la escena de Godot, y mantén tus propios scripts y nodos fuera de la rama generada.

## Unity
1. Exporta una vez dentro de la carpeta Assets del proyecto de Unity.
2. Haz clic en **Instalar complemento de Unity** y deja que Unity lo compile.
3. Abre Window → Heron Brush → Live Link en Unity.
4. Inicia la conexión y vincula el mapa a la escena activa de Unity.

Mantén abierta la ventana en vivo de Unity. Las actualizaciones se pausan en Play Mode y en Prefab Mode. Solo se sustituye el hijo Generated del objeto del mapa. No edites los archivos generados.

## Defold
1. Exporta una vez a tu proyecto de Defold y mantén el proyecto abierto en Defold.
2. Haz clic en **Iniciar conexión en vivo**. No hace falta ningún complemento.
3. Abre heron_brush_live/preview.collection en Defold, o haz referencia a ella desde tu propia escena.

La conexión escribe en una carpeta generada aparte y no toca la exportación final ni game.project. **Recargar el juego en ejecución** también actualiza un juego iniciado desde Defold; añadir objetos o cambiar la colisión puede requerir reiniciar el nivel. Los fallos de conexión se reintentan automáticamente.

## Flax
Exporta a la carpeta Content del proyecto y haz clic en **Instalar complemento de Flax**. Deja que Flax lo compile, inicia la conexión y, después, elige Tools → Heron Brush: attach live map en la escena de Flax abierta. La raíz generada conserva su colocación y los demás objetos no se tocan. Guarda tú mismo la escena. Las actualizaciones se pausan en Play Mode. Tools → Heron Brush: stop live map pausa la recepción.

# Sesiones colaborativas en vivo
Colaboración → Sesión en vivo abre una ventana para alojar a otro usuario de Heron Brush o unirte a él. Así se comparte la edición del mapa entre personas; es algo distinto de una conexión en vivo con un motor.

## Alojar y unirse
Escribe **Tu nombre**. Para alojar, elige **Crear una sesión**, haz clic en **Iniciar sesión** y envía a tus invitados los datos de **Copiar datos de conexión**. Para unirte, elige **Unirse a una sesión**, pega los datos y haz clic en **Unirse**. Al unirte, el mapa abierto se sustituye por el del anfitrión, así que guarda antes tu trabajo.

La versión estándar funciona en la red local, y **Puerto de red local** fija el puerto. Las sesiones por internet (**Conectar por internet**) requieren una versión con la función de red opcional. Los participantes ven en la vista 3D los marcadores de cámara y los nombres de los demás.

Al cerrar la ventana, la sesión sigue en marcha; el indicador de la barra de título muestra su estado. **Detener sesión** o **Salir de la sesión** la terminan. Cuando dos personas editan el mismo objeto, el orden del anfitrión decide la versión final, y deshacer también es una edición compartida. Trabajad en zonas separadas siempre que podáis y guardad puntos de control.

# Visualización 3D y un segundo monitor
## Modos de visualización
La sección **Vista** de la pestaña Mapa cambia lo que dibujan las vistas: Brushes, Entidades puntuales, Modelos de entidades, Nombres de clase de entidades, Contornos de grupos, Patches, Materiales, Mostrar caras omitidas, Previsualizar animaciones, Malla de alambre, Aristas y Contornos de entidades de pincel. Malla de alambre dibuja las aristas de los triángulos, incluidas las diagonales y las aristas situadas detrás de las superficies, sin materiales. Es una vista temporal de depuración y no afecta a las exportaciones ni a los renders.

La iluminación, las sombras, la oclusión ambiental, los reflejos, el bloom y el suavizado de la vista 3D están en Preferencias → Renderizador. Son ajustes de previsualización; las imágenes finales usan los ajustes propios de la cámara de render.

## Una ventana 3D independiente
Vista → Vista 3D en ventana independiente pasa la vista 3D a una ventana propia, que puedes colocar en otro monitor. Comparte la cámara, el documento y las herramientas, y las vistas planas ocupan el espacio en la ventana principal. Su menú ofrece Materiales, Vista sólida y Malla de alambre; un submenú Vista con Mostrar caras omitidas, Previsualizar animaciones, Aristas, Contornos de grupos, Entidades puntuales, Modelos de entidades y Contornos de entidades de pincel; Enfocar selección; Frontal, Lateral y Superior, que giran la cámara alrededor de la selección (o de un punto delante de la cámara); Colocar cámara desde vista 3D; Pantalla completa (`F11`); Renderizar imagen; y Volver a la ventana principal. Al cerrar la ventana, la vista vuelve a acoplarse.

## Cielo procedural
El cielo procedural está activado por defecto en la vista 3D y se puede desactivar en Preferencias → Renderizador → Luz. Las cámaras de render tienen su propio interruptor **Cielo procedural**, que se guarda con el mapa. Los colores del cielo, del horizonte y del suelo proceden de los ajustes de entorno, y el sol sigue la iluminación de la escena. El cielo no tiene geometría y no se exporta.

# Cámaras de render e imágenes PNG
Archivo → Renderizar imagen abre la ventana de render: la cámara y sus ajustes a la izquierda y, a la derecha, una vista previa en directo que se vuelve a dibujar a medida que los cambias. Las cámaras se guardan en el `.map`, en una capa Render cameras exclusiva del editor que queda fuera de las exportaciones a motores.

## Colocar la cámara
Encuadra la toma en la vista 3D y pulsa `Ctrl+Alt+0` (o `Ctrl+Alt+Num0`), o haz clic en **Colocar cámara desde vista 3D**. Si no hay ninguna cámara seleccionada, se actualiza la primera guardada; si no existe ninguna, se crea una. Se puede deshacer y conserva los ajustes de salida e iluminación. **Seleccionar cámara en el mapa**, **Cargar cámara del mapa** y **Mover vista 3D a la cámara** funcionan en sentido contrario. **Marco de cámara en vista 3D** muestra el límite de la imagen de salida, y **Controlar cámara desde vista 3D** permite que la navegación normal mueva la cámara guardada.

## Ajustes
- **Posición de cámara**: X, Y, Z, guiñada (yaw) y cabeceo (pitch).
- **Salida y objetivo**: **Preajustes** (Full HD, Cuadrado, Vertical), anchura, altura, muestras, **Campo de visión vertical** y **Plano de recorte lejano**.
- **Iluminación**: Luz diurna, Atardecer o Nublado, o bien ajusta la exposición, el sol, los colores del cielo, del horizonte y del suelo, **Cielo procedural**, **Sombras**, **Oclusión ambiental** y **Bloom**.
- **Profundidad de campo**: **Activar profundidad de campo**, distancia de enfoque, rango nítido y radio de desenfoque. **Enfocar selección** enfoca el centro de un objeto seleccionado situado delante de la cámara.

**Guardar cámara y ajustes** guarda el formulario en el mapa; guarda el mapa para conservarlo en el disco.

## Renderizar
**Renderizar PNG** pregunta dónde guardar e inicia un render progresivo, que se va completando muestra a muestra en la vista previa. Más muestras se ven mejor, pero tardan más. **Cancelar renderizado** se detiene sin escribir ningún archivo. El renderizador usa el pipeline de rasterización del editor, no path tracing. El PNG tiene un fondo opaco y las animaciones no se renderizan. Las imágenes muy grandes pueden detener el editor brevemente.

# Preferencias
Archivo → Preferencias (`Ctrl+,`) abre los ajustes. Los cambios se aplican de inmediato y se guardan. La barra lateral enumera Resumen, Juegos, Vista, Colores, Ratón, Teclado y Renderizador. El campo de búsqueda de la parte superior encuentra un ajuste por su nombre o su descripción en cualquier página. Cada ajuste modificado tiene su propio botón para restaurar su valor predeterminado, y **Restaurar los valores predeterminados** restablece la página actual.

## Las páginas
- **Resumen**: los ajustes habituales reunidos: Apariencia (Opacidad de los paneles (%), Esquema de colores, Disposición, Idioma), Edición, Vista y Manual.
- **Juegos**: el juego, dónde están sus archivos y tu nombre para la colaboración. Un juego indica qué formato de mapa escribir, qué definiciones de entidades cargar y dónde están sus materiales. Las configuraciones de juegos se leen de la carpeta `games` de la carpeta de configuración (`~/.config/heron-brush/games` en Linux y macOS, `%LOCALAPPDATA%\heron-brush\games` en Windows) y de la carpeta `games` junto a la aplicación, donde los paquetes colocan la configuración incluida.
- **Vista**: Disposición, Esquema de colores, Idioma, ejes del mundo, opacidad de la cuadrícula, tamaño de celda de la comprobación de fugas y los bloqueos de mapeado.
- **Colores**: los colores de las vistas, la geometría y los tiradores.
- **Ratón**: velocidades e inversiones para mirar, desplazar y mover, velocidad de vuelo, campo de visión, Enlazar las vistas 2D y tamaño de los tiradores.
- **Teclado**: el mapa de teclas, Heron Brush o Blender.
- **Renderizador**: sombreado realista, contorno de la selección, luz, sombras, oclusión ambiental, reflejos, bloom y superficies.

## Idioma
**Idioma** ofrece Seguir al sistema y todos los idiomas traducidos. La interfaz cambia al instante. El manual se abre en el idioma de la interfaz cuando existe una traducción, y en inglés en caso contrario.

# Complementos y MCP
## Complementos
Los complementos amplían el editor con scripts de Rhai. Archivo → Complementos abre la ventana Complementos. **Instalar un complemento** instala un archivo `.rhai` e **Instalar una carpeta de complemento** instala una carpeta que contiene `main.rhai`; los metadatos opcionales de `plugin.toml` describen el complemento y sus ajustes. Los complementos se agrupan en **Complementos activos** y **Complementos inactivos**, y **Activar complemento** activa o desactiva uno; esto se mantiene tras reiniciar. **Recargar complementos** vuelve a leer los scripts del disco y restablece los valores de sus paneles.

Los complementos activos también aparecen en la sección **Complementos** de la pestaña Mapa con sus ajustes, sus botones de acción (**Ejecutar**) y su salida. Una acción actúa sobre la selección o, si no hay nada seleccionado, sobre todo, y es un solo paso de deshacer. El Selection inspector incluido muestra recuentos y acciones para subir y bajar.

## MCP
Colaboración → Servir a un agente (MCP) permite que un agente de IA u otra herramienta controle el editor abierto mediante el Model Context Protocol. La ventana muestra la dirección local con **Copiar**, y **Detener servidor** lo termina; mientras se ejecuta, la barra de título muestra **MCP activo**. La opción de línea de comandos `--mcp` ejecuta la misma interfaz sin ventana. Los clientes pueden descubrir herramientas para geometría, assets, propiedades, animación, exportación y sesiones. MCP y los complementos del editor son interfaces distintas.

# Línea de comandos
El programa es `heron-brush`. Sin opciones, abre la pantalla de bienvenida.

## Abrir archivos
`heron-brush map.map [entities.fgd] [materials/]` abre un mapa, opcionalmente con definiciones de entidades y una carpeta de materiales. Un argumento vacío omite uno, por ejemplo `heron-brush map.map "" materials/`. Si ya hay un editor en ejecución, el mapa se abre en él; `--new-instance` abre en su lugar una ventana independiente.

## Comandos
- `--help` o `-h`: muestra el modo de uso. `--version` o `-V`: muestra la versión.
- `--mcp`: sirve MCP por la entrada y la salida estándar, sin ventana.
- `--merge-map <base> <ours> <theirs> [<output>]`: fusión a tres bandas de archivos `.map` para git. La salida es ours por defecto. Los conflictos se imprimen y devuelven un código de salida distinto de cero.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: convierte una malla en brushes. size es la longitud del lado más largo en unidades de mapa (256 por defecto), snap es una cuadrícula para las esquinas (0 para ninguna), y las palabras `solid` o `layered` crean cajas de cuadrícula o losas convexas en lugar de cuñas finas.
- `--render-preview <in.map> <out.png> [size]`: escribe una miniatura PNG de un mapa, de 512 píxeles por defecto.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: ejecuta un script de complemento sobre un mapa con todo seleccionado y escribe el resultado (por defecto, sobre el archivo de entrada). `key=value` responde a los ajustes del complemento.

# Solución de problemas
## Cuando algo no se ve bien
- Falta un material: comprueba la carpeta o el paquete de materiales y las rutas del proyecto de la exportación.
- Tamaño de modelo incorrecto: compáralo con el marcador del jugador de 64 unidades y con la escala de exportación.
- Dirección del degradado incorrecta: comprueba el ángulo del degradado y la orientación de la cara.
- Escena del motor vacía: lee los avisos de la exportación y sigue las instrucciones de importación del destino.
- Conexión en vivo en espera: comprueba que el complemento está instalado, que la escena está vinculada y que las actualizaciones no están en pausa.
- Solo líneas, sin caras: desactiva Malla de alambre en la sección Vista de la pestaña Mapa, o elige Materiales o Vista sólida en el menú de la ventana 3D.
- Faltan objetos en las vistas: `Ctrl+Shift+I` muestra los objetos ocultos; revisa las capas y los filtros de la sección Vista.
- Una tecla no hace nada: haz clic primero en la vista para que tenga el teclado, y revisa el mapa de teclas en Preferencias → Teclado.
- Una edición ha fallado: deshazla, simplifica la geometría y usa **Comprobar el mapa** y **Comprobar si hay leaks** en la sección **Problemas** de la pestaña Mapa.
- Un mapa no se abre desde el gestor de archivos: puede que se haya entregado a un editor que ya está en ejecución; busca esa ventana.

## Ajustes y copias de seguridad
Las preferencias, los mapas recientes, las estrellas, los ajustes de exportación y los complementos se guardan en la carpeta de configuración: `~/.config/heron-brush` en Linux y macOS, y `%LOCALAPPDATA%\heron-brush` en Windows. Las copias de seguridad automáticas están en la carpeta `autosave` junto a cada mapa.

## Informar de un error
Incluye la versión del editor (Ayuda → Acerca de), tu sistema operativo, un mapa pequeño que muestre el problema y los pasos exactos. El panel de salida (`Ctrl+4`) suele indicar qué ha fallado. Guarda una copia antes de hacer pruebas.

## Cómo usar este manual
Elige un capítulo en el índice de la izquierda. `Ctrl+F` muestra el índice y lleva al campo de búsqueda; `Esc` borra la búsqueda o, si está vacía, cierra el manual. **Contenido** oculta o muestra la lista de capítulos y **Tamaño de letra** cambia el tamaño de la fuente; ambos ajustes se recuerdan. La lista de idiomas de la parte superior cambia entre las traducciones disponibles.
