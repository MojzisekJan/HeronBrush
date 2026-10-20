# Primeiros passos
O Heron Brush é um editor de níveis baseado em brushes. Você constrói níveis a partir de brushes convexos, pinta e texturiza suas faces, posiciona entidades, anima portas e plataformas e exporta o resultado para uma engine de jogo. Ele também tem recursos embutidos, formas paramétricas, conexões ao vivo com editores de engines, sessões de edição compartilhada e renderização de imagens estáticas. Este manual acompanha o aplicativo e funciona offline.

No macOS, `Cmd` funciona onde quer que este manual diga `Ctrl`, e Option equivale a Alt.

## A tela de boas-vindas
Quando o Heron Brush é iniciado sem um mapa, a tela de boas-vindas se abre. À esquerda ficam **Continuar** (o último mapa aberto), **Novo mapa** e **Procurar** para abrir um `.map` existente, além de **Jogo**, **Manual** e **Preferências**. À direita há uma lista de mapas recentes com miniaturas, seguida de **Exemplos**. A estrela (**Fixar no topo**) fixa um mapa no topo da lista; a lixeira (**Mover para a lixeira**) move o arquivo para a lixeira do sistema, de onde ele pode ser restaurado. As setas do teclado percorrem a tela e Enter abre o item em foco. Arquivo → Boas-vindas traz a tela de volta a qualquer momento.

## Seu primeiro mapa
1. Escolha **Novo mapa** na tela de boas-vindas ou Arquivo → Novo mapa (`Ctrl+N`).
2. Escolha a ferramenta Brush (`B`) na barra de ferramentas e arraste em uma vista 2D para desenhar a forma selecionada. Confira a profundidade em outra vista 2D.
3. Selecione o brush e mova-o arrastando. Use a aba **Face** para dar a ele um material ou uma cor.
4. Salve o mapa de origem com `Ctrl+S`. Quando estiver pronto para usá-lo em um jogo, abra Arquivo → Exportar e gere uma cena para a sua engine.

## Arquivos e salvamento
O arquivo `.map` é a sua fonte editável. Salve com frequência: exportar, conexões ao vivo e renderizar nunca salvam o mapa por você. Arquivo → Salvar como (`Ctrl+Shift+S`) acrescenta a extensão `.map` se você a omitir. Arquivo → Reverter para o mapa salvo descarta as alterações não salvas e recarrega o arquivo. Se você fechar uma janela ou abrir outro mapa com alterações não salvas, o Heron Brush pergunta se deve **Salvar**, **Descartar** ou **Cancelar**.

Enquanto um mapa salvo tiver alterações não salvas, backups numerados são gravados a cada dez minutos em uma pasta `autosave` ao lado do mapa. Os 50 backups mais recentes são mantidos.

Abrir um mapa enquanto o Heron Brush já está em execução o entrega à janela aberta, que vem para a frente. Use `--new-instance` na linha de comando quando quiser uma janela separada (veja Linha de comando).

# A interface
A janela principal tem uma barra de título com os menus, uma barra de ferramentas logo abaixo, as vistas do mapa, o inspetor à direita e uma barra de status na parte inferior.

## Barra de título e menus
A barra de título mostra o logotipo, os menus, o nome do mapa no meio e os botões da janela. No macOS, os botões vermelho, amarelo e verde de sempre ficam à esquerda. Arraste a barra de título para mover a janela e clique duas vezes nela para maximizar. Enquanto algo é executado em segundo plano, um indicador aparece perto do fim da barra: uma conexão ao vivo com uma engine, uma sessão ao vivo ou **MCP ativo**. Clique em um indicador para parar a conexão ao vivo ou abrir a janela correspondente.

- **Arquivo**: Boas-vindas, Novo mapa (`Ctrl+N`), Abrir mapa (`Ctrl+O`), Salvar (`Ctrl+S`), Salvar como (`Ctrl+Shift+S`), Reverter para o mapa salvo, Exportar, Renderizar imagem, Plugins e Preferências (`Ctrl+,`).
- **Editar**: Desfazer (`Ctrl+Z`), Refazer (`Ctrl+Shift+Z`), Recortar, Copiar, Colar, Duplicar (`Ctrl+D`), Excluir, Selecionar tudo (`Ctrl+A`), Repetir últimos comandos (`Ctrl+R`) e Esquecer comandos repetíveis.
- **Vista**: Vista 3D em janela separada, Ir para, Ajustar à grade (`Alt+0`), Mostrar grade (`0`), Prévia de animações e Mostrar faces omitidas.
- **Geometria**: Inserir texto 3D, Unir canto, Converter faces em patches, Retirar a seleção de sua entidade e Sincronizar grupos vinculados.
- **Animação**: Animação por quadros-chave, Prévia de animações e Posicionar câmera a partir da vista 3D (`Ctrl+Alt+0`).
- **Mapa**: Carregar arquivo de pontos, Recarregar arquivo de pontos e Descarregar arquivo de pontos; Carregar arquivo de portais, Recarregar arquivo de portais e Descarregar arquivo de portais. Eles mostram os rastros de vazamento e os portais gravados pelos compiladores de mapa.
- **Colaboração**: Sessão ao vivo e Iniciar servidor para um agente (MCP).
- **Ajuda**: Manual e Sobre.

## A barra de ferramentas
A barra de ferramentas reúne todas as ferramentas. O primeiro grupo é usado o tempo todo: Selecionar, Brush, Recorte, Vértice, Aresta, Face, Girar e Escala. O segundo grupo traz Cisalhamento, Montar, Varredura, Ponto de controle, Arredondamento e Pintar. Passe o mouse sobre um botão para ver o nome e o atalho. Clicar de novo na ferramenta ativa volta para Selecionar.

Depois das ferramentas vêm **Inverter horizontalmente** e **Inverter verticalmente** e, em seguida, dois interruptores: **Bloquear** mantém o mapeamento do material quando um brush se move, e **UV** o mantém quando um brush gira (`U` alterna essa opção). Em seguida vêm as configurações da ferramenta ativa: a ferramenta Brush mostra a forma e suas opções, e a ferramenta Arredondamento mostra raio, faces e cantos com **Aplicar**. Na ponta direita ficam o nome da ferramenta, o tamanho da grade (**Grade 16** e assim por diante; clique para escolher outro) e o menu **Configurações da grade** com Ajustar à grade, Mostrar grade, Prévia de animações, Mostrar faces omitidas e Animação por quadros-chave.

## Vistas do mapa
Há quatro vistas: a vista 3D e três vistas planas, Topo (XY), Frente (XZ) e Lado (YZ). Preferências → Vista → Layout escolhe Um painel, Dois painéis, Três painéis ou Quatro painéis. Arraste os divisores para redimensionar os painéis. Clicar em uma vista dá a ela o foco do teclado. `Space` passa um painel que mostra uma vista por vez para a próxima vista, e `Ctrl+Space` maximiza a vista que tem o foco do teclado e depois restaura o layout.

Clique com o botão direito sem arrastar em qualquer vista para abrir um menu de contexto para a seleção e para o que está sob o ponteiro. Ele oferece agrupamento, camadas, grupos vinculados, criação de entidades, mover brushes para dentro ou para fora de uma entidade, copiar o nome de um material e mostrá-lo no navegador de materiais.

## O inspetor
O inspetor à direita tem quatro abas. Arraste a borda esquerda dele para mudar a largura.

- **Mapa** (`Ctrl+1`): Camadas, Grupos, Plugins, Problemas, filtros da seção Vista e Marcadores.
- **Entidade** (`Ctrl+2`): o navegador de entidades, as definições de entidades, as propriedades da entidade selecionada e, para botões e gatilhos, Acionador de animação.
- **Face** (`Ctrl+3`): UV, Materiais, Cor, Varredura, Arredondamento, Patch e Recuo e chanfro. Cada seção pode ser recolhida.
- **Recursos**: a biblioteca de modelos de brushes reutilizáveis.

`Ctrl+5` oculta ou mostra o inspetor, e `Ctrl+Alt+T` oculta ou mostra a barra de ferramentas, o que dá mais espaço às vistas.

## A barra de status e o painel de saída
A barra de status na parte inferior mostra a mensagem mais recente do editor. Erros aparecem em vermelho. Clique na barra ou pressione `Ctrl+4` para abrir o painel de saída com o histórico completo de mensagens. A maioria dos comandos informa ali o que fez, por exemplo quantos brushes foram recortados.

## Diálogos e aparência
Diálogos como Preferências, Exportar e Renderizar imagem abrem como janelas próprias. Preferências → Vista → Esquema de cores escolhe Seguir o sistema, Claro ou Escuro. **Opacidade dos painéis (%)** na página Visão geral deixa os painéis translúcidos sobre as vistas. A janela desenha os próprios botões no estilo do seu sistema: nativos no macOS, estilo Windows 11 no Windows e estilo GNOME no Linux.

# Navegação e teclado
## Movendo-se na vista 3D
Segure o botão direito do mouse e arraste para olhar ao redor. Arraste com o botão do meio para deslocar a vista. A roda avança e recua. `W`, `A`, `S` e `D` voam para a frente, para a esquerda, para trás e para a direita, `Q` sobe e `X` desce; segure `Shift` para passos maiores. Com **Alt + arrastar com o botão do meio move a câmera** ativado em Preferências → Mouse, Alt com o botão do meio move a câmera ao longo da direção da vista. Velocidades, inversões e campo de visão ficam em Preferências → Mouse.

## Movendo-se nas vistas 2D
Arraste com o botão do meio para deslocar a vista e use a roda para aplicar zoom em torno do ponteiro. Deslocar e aplicar zoom em uma vista plana mantém as outras alinhadas; **Vincular vistas 2D** em Preferências → Mouse controla isso. `Ctrl+Alt+Z` redefine o zoom e `Ctrl+Shift+U` leva as vistas planas de volta à origem.

`Ctrl+U` enquadra a seleção em todas as vistas. Vista → Ir para (`Ctrl+Alt+M`) leva a câmera a coordenadas digitadas, seleciona o que está em uma linha do arquivo do mapa ou move a seleção por um valor digitado.

## Grade e ajuste
`+` e `-` mudam o tamanho da grade, e as teclas numéricas de `1` a `9` escolhem um tamanho diretamente, de 1 a 256 unidades. `0` ou `#` mostra ou oculta a grade, e `Alt+0` liga ou desliga o ajuste à grade. `S` ajusta os vértices dos brushes selecionados à grade atual, `Ctrl+Shift+V` os ajusta a unidades inteiras e `Ctrl+Alt+Shift+V` à grade.

## Atalhos comuns
- `Ctrl+S`: salvar o mapa. `Ctrl+Shift+S`: salvar como.
- `Ctrl+Z`: desfazer. `Ctrl+Shift+Z` ou `Ctrl+Y`: refazer.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: copiar, recortar e colar. Colar põe os objetos no ponteiro; `Ctrl+Alt+V` mantém as coordenadas originais. Com faces selecionadas, copiar pega o material e as configurações de UV delas.
- `Ctrl+D`: duplicar. `Ctrl+Shift+D`: duplicata vinculada de um grupo.
- `Delete` ou `Backspace`: excluir a seleção.
- `Esc`: cancelar o arraste atual, voltar um passo na ferramenta ativa, voltar para Selecionar e, por fim, limpar a seleção. `Shift+Esc` volta direto para Selecionar.
- `Ctrl+R`: repetir os últimos comandos repetíveis. `Ctrl+Shift+R`: esquecê-los.
- `Ctrl+Alt+0` ou `Ctrl+Alt+Num0`: posicionar a câmera de renderização na vista 3D.
- `F5`: recarregar os materiais. `F6`: recarregar as definições de entidades.

## Atalhos de seleção
- `Ctrl+A`: selecionar tudo ou, com uma ferramenta de vértices, arestas ou faces ativa, todas as alças. `Ctrl+Shift+A`: não selecionar nada. `Ctrl+Alt+A`: inverter a seleção.
- `Ctrl+B`: selecionar os outros objetos da mesma entidade ou grupo.
- `Ctrl+T`: selecionar tudo o que os brushes selecionados tocam. `Ctrl+E`: selecionar tudo o que está dentro deles. `Ctrl+Shift+E`: selecionar tudo na coluna deles ao longo do eixo da vista. Esses comandos consomem os brushes usados para selecionar.
- `Ctrl+I`: isolar a seleção. `Ctrl+Alt+I`: ocultá-la. `Ctrl+Shift+I`: mostrar tudo de novo.
- `Ctrl+G`: agrupar. `Ctrl+Shift+G`: desagrupar.

## Atalhos de transformação
- As setas e `Page Up` / `Page Down` movem a seleção um passo da grade na direção que apontam na vista ativa. Com `Ctrl` elas duplicam enquanto movem e, com `Alt`, giram.
- `Alt+X`, `Alt+Y`, `Alt+Z`: girar um quarto de volta em torno desse eixo; acrescente `Shift` para o outro sentido.
- `Ctrl+F`: inverter na horizontal da vista; `Ctrl+Alt+F`: inverter de cima para baixo. `Ctrl+Shift+X` e `Ctrl+Shift+Y` espelham ao longo de X ou Y.
- `Insert`: posicionar uma entidade pontual no centro da seleção.
- `Tab`: com um brush selecionado, alternar Selecionar → Vértice → Aresta → Face → Selecionar.

## O mapa de teclas do Blender
Preferências → Teclado → Mapa de teclas oferece Heron Brush (o padrão, usado em todo este manual) e Blender. Com Blender: `G` seleciona e move, `R` gira, `S` escala, `E` faz varredura, `1`, `2` e `3` escolhem as ferramentas de vértice, aresta e face, `K` recorta, `B` seleciona, `A` seleciona tudo e `Alt+A` desfaz a seleção, `X` exclui, `Shift+D` duplica, `H` oculta e `Alt+H` mostra tudo, e `Ctrl+Shift+Z` refaz. Todo o resto é igual nos dois.

# Brushes e ferramentas de modelagem
Brushes são sólidos convexos definidos por planos. Construa objetos complexos a partir de vários brushes simples em vez de tentar fazer um único brush côncavo. Mantenha o número de brushes baixo onde uma forma simples basta.

## Desenhando brushes
Escolha a ferramenta Brush (`B`) e arraste em uma vista. A barra de ferramentas mostra **Forma para desenhar**: Paralelepípedo, Escada, Arco (abóbada), Cilindro, Cone, Esferoide (UV), Esferoide (icosaedro) e Inserir texto 3D. `Alt+1` a `Alt+7` escolhem uma forma diretamente. Dependendo da forma, a barra mostra **Criar cavidade** e **Espessura:**, **Altura dos degraus:** e **Direção:** para escadas, **Eixo:**, **Lados:** e o modo de círculo para formas redondas, **Anéis:** para esferas e **Precisão:** para o icosaedro. Defina as opções antes de arrastar. Confira o resultado em mais de uma vista.

## Selecionando
Clique em um brush para selecioná-lo e clique com `Ctrl` para adicioná-lo à seleção. Arrastar a partir de um espaço vazio desenha um retângulo de seleção. Na vista 3D, clicar com `Shift` seleciona uma única face e clicar com `Ctrl+Shift` adiciona mais faces. Arrastar um brush selecionado o move; arrastar um não selecionado seleciona e move apenas esse.

Na vista 3D, arraste com `Shift` uma face de um brush selecionado para extrudá-la. Clique com `Alt` em outra face para copiar para ela o material e o alinhamento da face selecionada; `Alt+Ctrl` copia só o material e `Alt+Shift` copia a rotação.

## Ferramentas de modelagem
- **Selecionar** (`Esc`): selecionar e mover.
- **Vértice** (`V`), **Aresta** (`E`), **Face** (`F`): arrastar cantos, arestas ou faces. Arrastar com `Shift` a partir do meio de uma aresta ou face adiciona um canto ali. `Delete` remove os cantos escolhidos.
- **Girar** (`R`), **Escala** (`T`), **Cisalhamento** (`G`): transformar a seleção com alças.
- **Recorte** (`C`): clique em dois ou três pontos para definir um plano de corte. `Enter` corta, `Ctrl+Enter` escolhe qual lado manter, e `Backspace` ou `Esc` remove o último ponto.
- **Montar** (`N`): posicione pontos ao redor de um volume e pressione `Enter` para criar um brush convexo a partir deles. `Backspace` remove o último ponto.
- **Varredura** (`Y`): veja a seção sobre varredura abaixo.
- **Ponto de controle** (`P`): remodelar a grade de controle de um patch.
- **Arredondamento** (`O`): clique em uma aresta para arredondá-la ou escolha arestas ou vértices com as ferramentas correspondentes e pressione `Enter`. Mais faces dão um resultado mais suave e mais geometria.
- **Pintar** (`K`): veja Materiais e UVs.

## Combinando brushes
- `Ctrl+J`: mesclar os brushes selecionados em seu fecho convexo.
- `Ctrl+K`: subtrair os brushes selecionados dos brushes em que eles tocam.
- `Ctrl+Shift+K`: tornar a seleção oca, com paredes da espessura de um passo da grade.
- `Ctrl+L`: interseção da seleção.

## Varredura, arredondamento, patch e chanfro
**Varredura** cria geometria ao longo de uma trajetória a partir das faces selecionadas. Selecione as faces (na ferramenta Varredura, clique com `Alt`), depois defina **Deslocamento**, **Rotação** (eixo e ângulo), **Escala**, **Passos** e **Trajetória** (alinhamento Inteiro ou Livre; Arco, Reta ou Curva em S) na seção Varredura da aba Face e pressione **Varredura**. A linha abaixo dos controles diz quantos brushes serão criados.

A seção **Arredondamento** arredonda as arestas escolhidas com **Raio**, **Faces** e **Cantos** (Meia-esquadria ou Arredondado); uma face só produz um chanfro.

**Patch** mostra **Colunas** e **Linhas** do patch selecionado, com botões para adicionar ou remover um par. Geometria → Converter faces em patches (`Ctrl+P`) cria patches a partir das faces selecionadas.

**Recuo e chanfro** atua nas faces selecionadas: **Recuo** encolhe a face a partir das bordas e **Profundidade** a empurra para fora (ou para dentro, quando negativa). **Chanfrar faces** aplica os dois; o terceiro botão adiciona um painel ou abre um rebaixo, conforme o sinal da profundidade. **Unir canto** abre um diálogo que une dois brushes em um canto em meia-esquadria ou arredondado, com prévia.

## Texto 3D
Geometria → Inserir texto 3D (também o último item da lista de formas da ferramenta Brush) cria brushes a partir de texto. Ele usa a Heron Block, uma fonte poligonal embutida com cantos chanfrados, larguras proporcionais e acentos do tcheco; as letras minúsculas usam as formas maiúsculas. Digite o **Texto**, defina **Altura** (a altura das maiúsculas; os acentos ficam acima dela) e **Profundidade**, acompanhe a prévia ao vivo e pressione **Inserir**.

Se uma operação não conseguir produzir geometria válida, ajuste as dimensões ou desfaça e simplifique a forma.

## Matrizes
Geometria → Matriz copia a seleção e mostra as cópias no mapa enquanto você altera os valores. As quantidades incluem o original, e uma matriz cria no máximo 1024 cópias.

- **Fileira** desloca cada cópia em relação à anterior em **Passo X**, **Passo Y** e **Passo Z** e a gira **Giro por cópia (graus)** em torno do eixo escolhido.
- **Grade** preenche **Quantidade X**, **Quantidade Y** e **Quantidade Z** itens, espaçados por **Passo X**, **Passo Y** e **Passo Z**.
- **Anel** distribui os itens por **Ângulo (graus)** em volta de um centro a **Raio (unidades)** da seleção. Com **Voltar para o centro**, cada cópia gira junto com o anel.
- **Dispersão** coloca as cópias em lugares aleatórios dentro do raio, atravessando o eixo escolhido. A mesma **Semente** dá a mesma disposição, e **Giro aleatório** gira cada cópia.

Quando a seleção é um grupo inteiro, **Cópias vinculadas** torna cada cópia uma duplicata vinculada, então uma alteração em uma chega a todas. **Aplicar** mantém as cópias e as seleciona junto com o original; **Cancelar** as remove. A matriz inteira é desfeita em um único passo.

## Esculpir terreno
O terreno é feito de muitos brushes pequenos cujo topo o pincel de esculpir sobe e desce. Selecione uma caixa plana e escolha Geometria → Dividir em colunas: a caixa é cortada em colunas triangulares com um passo de grade de largura, colocadas em um grupo chamado Terrain. O topo de um triângulo é sempre plano, então as colunas podem formar cristas e vales.

Escolha a ferramenta **Esculpir** (`J`) e arraste sobre o terreno na vista 3D. O anel laranja mostra o pincel. Sem seleção, o pincel altera todos os brushes abaixo dele; com seleção, só os selecionados. A barra no topo da janela escolhe o que um traço faz:

- **Elevar** e **Baixar** movem a superfície para cima ou para baixo, mais no centro do anel e nada na borda. Segure `Shift` para trocá-los.
- **Suavizar** nivela as saliências.
- **Aplainar** iguala a superfície à altura onde o traço começou.

Os dois números ao lado dos modos são **Raio (unidades)** e **Força (%)**. Só os cantos de cima se movem, na vertical; a base fica onde está e uma coluna mantém pelo menos uma unidade de espessura. Um traço é desfeito em um único passo. Nas vistas 2D a ferramenta seleciona como a ferramenta Selecionar.

# Materiais e UVs
## Carregando e aplicando materiais
Abra a seção **Materiais** da aba Face. **Carregar pasta** adiciona uma pasta de imagens e **Carregar .wad** adiciona um pacote de texturas; ambos aparecem em **Coleções**, onde o X remove um deles. Digite em **Filtrar materiais** para restringir a lista. Clique em um material para torná-lo o atual e **Aplicar à seleção** o coloca em todas as faces selecionadas. **Selecionar** oferece Selecionar brushes que o usam e Selecionar faces que o usam. **Substituir** troca um material por outro no mapa inteiro ou só na seleção.

A ferramenta **Pintar** (`K`) aplica o material atual à face sob o ponteiro: um clique pinta uma face, um clique duplo pinta o brush inteiro e um clique com `Shift` pega o material da face como atual, como um conta-gotas.

## O editor de UV
A seção **UV** da aba Face mostra o material e o contorno da face selecionada. Controles do mouse na área de edição:
- A roda aplica zoom em torno do ponteiro. Arraste com o botão direito ou do meio para deslocar. Clique duas vezes com o botão direito para enquadrar a face de novo; selecionar outra face a enquadra automaticamente.
- Arrastar com o botão esquerdo move o material enquanto o contorno da face fica no lugar. O anel gira o mapeamento, as alças dos cantos o escalam e a alça central define a origem dessas mudanças.
- Arrastar com `Alt` e o botão esquerdo sobre a borda de um ladrilho aplica cisalhamento, assim como arrastar com `Shift` e o botão do meio.
- Cada arraste é um passo de desfazer, e `Esc` cancela um arraste em andamento.

Abaixo da área de edição ficam **Deslocamento**, **Escala**, **Ângulo** e, quando o jogo os usa, **Flags de conteúdo**, **Flags de superfície** e **Valor de superfície**. Os botões são **Redefinir**, **Alinhar ao mundo**, **Ajustar**, **Inverter H**, **Inverter V** e as duas rotações de 90°. **Alinhar** gira o material ao longo de uma das arestas da face (pressione de novo para a próxima aresta, com `Shift` para a anterior). Os quatro botões de justificação encostam o material em uma borda. **Ajustar U** e **Ajustar V** escalam para que um número inteiro de repetições cubra a face; pressione de novo para a próxima contagem e segure `Ctrl` para manter a subdivisão de uma trim sheet.

## Teclas de UV
Com faces selecionadas, as setas movem o material, `[` e `]` o giram em 15°, `,` e `.` reduzem a escala à metade ou a dobram, `Ctrl+F` e `Ctrl+Alt+F` o invertem, `Shift+R` o redefine, `Alt+Shift+R` o alinha aos eixos do mundo e `Ctrl+Shift+F` o ajusta a cada face.

**Bloquear** na barra de ferramentas mantém o mapeamento enquanto os brushes se movem, e **UV** (`U`) o mantém enquanto os brushes giram.

## Materiais de ferramentas
- skip: remove a face marcada das exportações para engines.
- clip: cria colisão sem geometria visível.
- origin: marca o pivô de uma entidade brush.

Os nomes podem ser alterados no cartão **Materiais de ferramentas** da janela Exportar. Use skip em faces que você sabe que ficam escondidas.

## Omitindo faces da exportação
Selecione uma face e ative **Omitir face na exportação** na seção UV. Uma hachura laranja a marca na prévia de UV. O material e a pintura continuam salvos, e o brush continua fechado e editável. As malhas de renderização exportadas deixam essa face de fora, mas a colisão é mantida. Deixar só um quadrilátero de um cubo produz dois triângulos de renderização. Isso não torna as faces de dupla face nem mescla malhas.

**Omitir faces cobertas** marca faces totalmente cobertas por brushes estáticos, incluindo faces idênticas encostadas e brushes envoltos por outros. Atua nos brushes selecionados ou, quando nada está selecionado, em toda a geometria de mundo visível e desbloqueada. Entidades brush, materiais transparentes e brushes que já têm faces omitidas são ignorados. Faces parcialmente cobertas não são divididas. Um único desfazer restaura tudo. Vista → Mostrar faces omitidas oculta ou mostra essas faces nas vistas do editor; a seleção continua usando o brush inteiro.

# Cor e gradientes
A seção **Cor** da aba Face pinta faces com cores sólidas ou gradientes por cima dos materiais. É útil para fazer o blockout de um nível e para arte estilizada. Faces pintadas mantêm suas cores reais nas vistas; a seleção é indicada pelo contorno.

## Paleta e cor de mistura
Clique em uma das doze amostras da paleta para pintar com ela as faces selecionadas ou brushes selecionados inteiros. **Opacidade** define, em porcentagem, a opacidade da próxima pintura. **Misturar** tem um botão de cor que abre o seletor de cores; arraste no seletor e a seleção acompanha ao vivo, ou pressione **Pintar** para aplicar a cor de mistura em um único passo. **Voltar ao material** remove a pintura para que a face volte a mostrar o material. A linha no topo da seção diz o que está selecionado e o que está aplicado.

## O seletor de cores
Todo botão de cor do editor abre o mesmo seletor. Arraste no quadrado para escolher saturação e brilho, e na faixa ao lado para escolher o matiz. Abaixo ficam os valores **R**, **G** e **B** e um controle deslizante de **Opacidade** quando a cor tem uma. O campo hexadecimal aceita `#RRGGBB` ou `#RRGGBBAA`; ele fica vermelho enquanto o texto não for uma cor válida. **Copiar** coloca o código hexadecimal na área de transferência. Clique fora do seletor para fechá-lo.

## Gradientes
**Gradiente** escolhe Gradiente linear ou Gradiente radial, e o botão ao lado inverte as cores de ponta a ponta. Cada ponto tem um botão de cor e **Posição (%)**, e os pontos entre as pontas têm um botão de remover. **Adicionar ponto de cor** adiciona um ponto no maior intervalo; um gradiente pode ter até oito pontos. **Ângulo do gradiente** define a direção de um gradiente linear, com os botões **+45°** e **−45°**.

Abaixo da prévia há um trilho com um marcador para cada ponto. Clique no trilho para adicionar um ponto, clique em um marcador para abrir o seletor dele, arraste um marcador para movê-lo e clique com o botão direito em um marcador para removê-lo. As três amostras abaixo do trilho são predefinições: Gradiente quente, Gradiente frio e Gradiente em cinza. As alterações em um gradiente se aplicam à seleção ao vivo.

A direção de um gradiente importa: oriente-o ao longo da inclinação de um telhado ou ao longo de um objeto curvo, não atravessando faces sem relação entre si. Confira o modelo dos dois lados depois de pintar.

## Material da superfície
**Material da superfície** define **Rugosidade** (0 é brilhante, 100 é fosco), **Metalicidade** (0 para madeira, pedra ou plástico, 100 para metal) e **Emissão** (de 0 a 8; a superfície brilha com a própria cor, incluindo gradientes, mas não ilumina objetos próximos). As alterações se aplicam ao vivo às faces ou brushes selecionados. **Usar superfície padrão** remove a substituição. Todas as alterações podem ser desfeitas.

Os efeitos de superfície aparecem na vista 3D iluminada e nas renderizações de câmera. As exibições plana e em aramado deixam os reflexos especulares de fora de propósito. O bloom pode adicionar um brilho em torno das superfícies emissivas. Godot e glTF/GLB recebem os valores do material, e o importador do Unity converte rugosidade em suavidade (smoothness). Veja `docs/surface-materials.md` para os limites de cada destino.

# Recursos, camadas e grupos
## A biblioteca de recursos
Abra a aba **Recursos**. Adicione uma pasta de modelos `.map` com **Adicionar pasta**; a biblioteca embutida está sempre disponível. Digite em **Filtrar recursos** para pesquisar. Clique em um modelo para selecioná-lo e então clique duas vezes nele, pressione `Enter` ou clique em **Inserir** para adicioná-lo onde o ponteiro esteve por último. Você também pode arrastar um modelo para uma vista. Os brushes de um recurso são inseridos como um único grupo, então o modelo se move como um todo.

**Salvar seleção como recurso** grava os brushes selecionados na biblioteca como um `.map`. Recursos da biblioteca são brushes editáveis comuns, não malhas importadas.

## Camadas
A lista **Camadas** da aba Mapa organiza áreas grandes. Digite um nome e clique em **Adicionar** para criar uma camada, ou pressione `Enter` para renomear a atual. Cada camada pode ser ocultada, bloqueada ou deixada fora das exportações. **Mover para cá** move a seleção para a camada atual. O menu de contexto nas vistas também oferece Mover para camada, Definir como camada ativa, Ocultar camadas e Isolar camadas.

## Grupos e grupos vinculados
`Ctrl+G` agrupa a seleção e `Ctrl+Shift+G` a desagrupa. A seção **Grupos** da aba Mapa lista os grupos e oferece **Abrir**, **Renomear**, **Mesclar**, **Desagrupar**, **Retirar** e **Extrair**. Um grupo vinculado é um conjunto de cópias que compartilham edições: `Ctrl+Shift+D` cria uma duplicata vinculada, Geometria → Sincronizar grupos vinculados atualiza as cópias e **Desvincular** torna uma cópia independente. Na aba Entidade, uma propriedade pode ser mantida como própria de uma cópia quando os grupos vinculados são sincronizados.

## Uma escala comum
A escala de exportação padrão é de 32 unidades de mapa por metro. O marcador de início do jogador embutido tem 64 unidades de altura, uma referência de 2 metros. Compare portas, veículos, móveis e plantas com ele. Mude a escala de exportação só de propósito e mantenha os recursos relacionados consistentes.

# Entidades e jogabilidade
As definições de entidades mais comuns vêm embutidas e estão sempre disponíveis: luzes, inícios de jogador, portas, botões, plataformas, gatilhos, objetos de cenário estáticos, sons e waypoints. Carregar um FGD do projeto com **Definições de entidades** na aba Entidade substitui as classes de mesmo nome e mantém as demais embutidas.

## Posicionando entidades
No navegador **Entidades** da aba Entidade, digite em **Filtrar classes** e escolha uma classe. Para uma classe pontual, como uma luz, clique em **Posicionar** ou clique duas vezes na classe; `Insert` posiciona uma no centro da seleção. Para uma classe brush, como uma porta ou um gatilho, selecione os brushes e clique em **Criar**. As classes também podem ser arrastadas para uma vista.

## Propriedades
A lista de propriedades edita a entidade selecionada. Entidades da mesma classe são editadas juntas. **Adicionar propriedade** adiciona uma chave que a definição pode não conhecer; **Padrões** pode devolver as propriedades existentes aos valores padrão ou adicionar as que faltam. Propriedades que a classe define, mas a entidade não tem, aparecem esmaecidas; digitar um valor as adiciona. Use targetname e target para ligar entidades.

## O que uma exportação pode fazer
As diretivas do Godot criam nós nativos, corpos de colisão e áreas de gatilho. Unity e Unreal suportam as luzes comuns de forma nativa, e o Unreal também cria atores PlayerStart. Os demais dados são preservados até onde o importador do destino permitir.

Uma entidade de porta sozinha não adiciona comportamento de jogo. Movimento, respostas a gatilhos, controles do jogador e scripts do projeto ainda precisam ser feitos na engine. O Defold exporta propriedades de entidades e objetos de colisão; o Flax posiciona modelos e entidades genéricas. Confira o resultado na sua engine antes de construir a jogabilidade em cima dele.

# Animação
O Heron Brush pode pré-visualizar movimentos simples e editar animações por quadros-chave de entidades brush, e depois exportá-las para várias engines.

## Prévia de animações
Ative **Prévia de animações** no menu Vista, no menu Animação, no menu Configurações da grade ou na seção Vista da aba Mapa. Ela reproduz a rotação de func_rotate e o vaivém de func_shuttle quando start_on está ativado, usando as propriedades de eixo, velocidade, direção, percurso e espera da entidade, e reproduz clipes de quadros-chave configurados com Reproduzir ao iniciar. Os recursos de ventilador e de moinho já têm partes giratórias.

Desligar a prévia restaura a pose original. Um clique em uma vista para a reprodução antes de selecionar ou editar; a navegação da câmera continua funcionando. A prévia não altera o mapa, o histórico de desfazer nem as exportações, e não executa física nem lógica de jogo.

## O editor de quadros-chave
Abra-o com Animação → Animação por quadros-chave ou pelo menu Configurações da grade. Ele fica acoplado abaixo das vistas ou na janela 3D quando ela está separada. O ícone de janela no cabeçalho o move para uma janela própria; o quadro e o histórico de desfazer são mantidos. **Editar animação** troca as vistas para a pose animada e **Editar mapa** volta para a geometria em repouso. No modo de animação, cada vista mostra **Editar animação** e o número do quadro na parte inferior.

Selecione uma entidade brush, ou um brush ou grupo de brushes, para animá-lo. Um grupo de brushes simples compartilha uma animação e um pivô; grupos que contêm várias entidades precisam de animações separadas.

## Clipes
O seletor **Animação** alterna entre clipes nomeados. Digite um nome em **Nome da animação** e clique em **Salvar como** para copiar o clipe atual com esse nome; **Excluir** remove um clipe. **Transladar** cria um clipe `translate` sem rotação. **Animações de porta** cria clipes suaves `open` e `close` com um giro de 90 graus em torno do pivô atual; mova o pivô para a dobradiça primeiro. **Reproduzir ao iniciar** faz o clipe tocar quando o nível começa; a predefinição de porta desativa essa opção.

## Transporte e reprodução
Os botões de transporte são **Início**, **Chave anterior**, **Reprodução**, **Próxima chave** e **Fim**. **Quadro** mostra e define o quadro atual, e **Duração (s)** define a duração do clipe. O modo é **Uma vez**, **Repetição** ou **Ida e volta**.

## A linha do tempo
A linha do tempo tem uma régua com o cursor de reprodução e uma faixa para **Posição** (X, Y e Z com chaves em conjunto) e para cada ângulo de rotação, RX, RY e RZ. Clique ou arraste na régua para mover o cursor de reprodução e pré-visualizar essa pose em todas as vistas. Arraste com o botão do meio para deslocar a linha do tempo. Arraste as chaves para movê-las no tempo; `Shift` adiciona à seleção; clique duas vezes em uma faixa para inserir uma chave; `Ctrl`+roda aplica zoom. **Curvas** mostra as curvas de valores em vez das faixas.

Os botões acima dela são **Adicionar chave**, **Remover**, **Copiar**, **Colar** e **Duplicar**. **Ajustar aos quadros** mantém as chaves em quadros inteiros e **Ajustar** mostra o clipe inteiro. **Ajustar tempos das chaves** abre **Escala de tempo**, **Deslocamento (quadros)**, **Escalar a partir do cursor**, **Estender duração** e **Ajustar tempos da seleção**.

O inspetor de chaves ao lado da linha do tempo edita a chave selecionada: **Quadro**, **Valor** (ou as três coordenadas de uma chave de Posição), a interpolação **Linear**, **Constante** ou **Bézier**, **Inclinação de entrada** e **Inclinação de saída**, e **Suavizar tangentes**.

## Teclas no editor
Enquanto a linha do tempo tem o foco do teclado: `Space` reproduz ou pausa, as setas para a esquerda e para a direita avançam um quadro, `Ctrl+A` seleciona todas as chaves, `Ctrl+C`, `Ctrl+X` e `Ctrl+V` copiam, recortam e colam chaves, `Ctrl+D` as duplica, `Delete` as remove, `Ctrl+Z` desfaz e `Esc` cancela um arraste. Fora da linha do tempo, `Esc` sai do modo de animação. Desfazer mantém o zoom, a seleção e o cursor de reprodução onde estavam.

## Criando poses nas vistas
A linha abaixo da linha do tempo tem **Casca de cebola** com uma contagem e **Espaçamento**, que mostra poses próximas como fantasmas; as coordenadas de **Pivô**; **Chaves automáticas**; e as ferramentas de pose **Transladar**, **Girar** e **Pivô**.

- **Adicionar chave** ativa Chaves automáticas e pausa a reprodução. Com Chaves automáticas ativadas e **Transladar** escolhido, arraste um eixo colorido da alça da entidade selecionada na vista 3D para criar uma chave de posição no quadro atual.
- Em uma vista 2D, arraste o brush selecionado ou um membro do grupo dele para criar chaves de translação ao longo dos dois eixos visíveis, com ajuste à grade. Chaves automáticas não precisa estar ativado para isso.
- Com **Girar**, arraste o brush em uma vista 2D em torno do marcador laranja do pivô. Topo cria chaves em RZ, Frente em RY e Lado em RX, ajustadas aos ângulos da grade.
- Com **Pivô**, arraste o centro de rotação amarelo em qualquer vista 2D. Ele se ajusta à grade e mantém a profundidade. As chaves e a geometria em repouso não mudam.

A trajetória ciano em 2D e 3D mostra o percurso do pivô. Clique em um ponto da trajetória para selecionar o tempo e as chaves dele, ou arraste-o para mudar a posição; com Chaves automáticas ativado, você também pode arrastar pontos da trajetória na vista 3D. Rótulos mostram os números dos quadros e segmentos tracejados marcam saltos. `Esc` cancela qualquer um desses arrastes, e um único desfazer reverte um arraste inteiro.

Animações novas começam em repouso, com chaves no tempo zero. Crie movimento mudando a pose em outro quadro. Para uma trajetória em L com canto vivo, crie chaves no início, no canto e no fim, e use Linear com Ida e volta.

## Ligando botões a animações
Selecione um func_button, trigger_multiple ou trigger_once. A aba Entidade mostra **Acionador de animação** acima das propriedades. Escolha um alvo na lista, ou clique em **Escolher alvo na vista** e depois em um objeto animado em qualquer vista; `Esc` cancela a escolha. Alvos ou clipes ausentes são informados abaixo dos controles. As ligações entre entidades mostram setas até os alvos quando a exibição de ligações está ativada.

**Ação** escolhe o que a ativação faz: **Reproduzir** inicia um clipe nomeado ou a animação selecionada; **Alternar** alterna entre `open` e `close` quando eles existem e, caso contrário, reinicia a animação selecionada; **Abrir** e **Fechar** iniciam esses clipes; **Pausar** congela a pose, **Retomar** continua e **Parar** restaura a pose original. **Apenas uma vez** substitui o padrão da classe. **Intervalo de repetição (s)** define o tempo mínimo entre ativações; ele não atrasa a primeira.

**Testar acionador** executa a ação na prévia sem editar o mapa nem adicionar passos de desfazer. **Redefinir prévia** restaura as poses originais e limpa o estado de uso único e de intervalo. Isso não simula colisões do jogador nem scripts do jogo.

## Animação nas exportações
O Godot exporta recursos nativos AnimationPlayer e AnimationLibrary, incluindo curvas e colisão em movimento. Um CharacterBody3D que entra na área de um botão ativa o alvo dele. Edições feitas nesses clipes no Godot não voltam para o mapa e são substituídas na próxima exportação.

A exportação para raylib adiciona `<map>_animation.h` com um player em C e `<map>_preview.c` com uma prévia de câmera livre; siga `<map>_raylib.md` para compilá-la. Na prévia, `SPACE` pausa, `R` reinicia os clipes padrão e `N` alterna entre os clipes nomeados. O Defold incorpora a reprodução ao script da entidade com mensagens `heron_*_animation` documentadas. O Build Level do Unity adiciona `HeronBrushAnimation` e um corpo cinemático. A exportação para Bevy inclui `heron_animation.rs` e instruções. Godot, Unity, Defold, Bevy e raylib mantêm as ligações de gatilho, o sinalizador de uso único e o intervalo de repetição. O seu jogo ainda precisa registrar o player e ligar os próprios botões à API de cada runtime.

## O exemplo Motion lab
Abra `motion_lab` em Exemplos na tela de boas-vindas. Ele tem uma plataforma com trajetória em L, um elevador, uma plataforma giratória e uma porta com dobradiças e botões. Abra no Godot o `godot/project.godot` que está ao lado e pressione `F5`. WASD e o mouse movem o jogador, Espaço pula, 1 a 4 levam às estações e R reinicia a estação atual. Pise em uma placa colorida para ativar o alvo dela. O README incluído explica como exportar novamente.

# Exportando para uma engine
Arquivo → Exportar abre a janela Exportar. As engines ficam listadas à esquerda com seus logotipos: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb e, por último, `.map`. A página à direita diz o que o destino vai receber. **Exportar** grava a exportação, **Verificar** monta o nível sem gravar e lista os problemas, e **Fechar** fecha a janela.

Exportar é separado de salvar: o `.map` continua sendo a sua fonte editável. Cada controle salva as configurações da sua engine assim que muda, então cada engine guarda as próprias respostas e a janela abre como você a deixou.

## Opções comuns
- **Escala**: **Unidades por metro**, 32 por padrão.
- **O que exportar**: **Geometria de colisão**.
- **Otimizações**: **Agrupar malhas estáticas em células de 1024 unidades**, **Remover faces ocultas dentro da geometria**, **Mesclar faces coplanares** e **Mesclar formas de colisão**.
- **Caminhos do projeto**: onde o nível e seus recursos ficam dentro do projeto.
- **Materiais de ferramentas**: os nomes dos materiais clip, skip e origin.
- **Mais**: opções específicas da engine, como **Nomear nós a partir de**, **Recriar camadas e grupos como nós pais**, **Inverter a ordem dos vértices dos triângulos**, inversão de coordenadas de textura e um segundo conjunto de UV para iluminação pré-calculada com **Texel de lightmap**.
- **Acompanhar alterações do mapa**: **Exportar novamente sempre que o mapa for salvo**. Fica disponível depois que a engine foi exportada uma vez, porque essa exportação diz onde gravar.

As células estáticas atribuem os brushes do mundo pelo centro e os combinam por material dentro de cada célula de 1024 unidades, para que as engines possam descartar partes menores do mapa. A colisão é mantida e as entidades de jogabilidade continuam separadas. Isso pode significar mais draw calls do que uma única malha de mundo, por isso vem desativado por padrão. Exportações avulsas de OBJ, glTF e GLB não usam células.

## Notas sobre os destinos
- Godot: caminhos `res://` de texturas e materiais, a camada de física dos gatilhos e o UV2 para lightmaps. Escolha uma pasta de exportação dentro do projeto Godot.
- Defold: o diretório de recursos dos assets, o material do modelo e o sampler de textura, que precisa existir no material escolhido.
- Unity: malhas OBJ e um importador para o editor; exporte para dentro da pasta Assets do projeto.
- Unreal Engine: um script de importação gerado e um caminho de conteúdo, normalmente /Game/<level>.
- Flax Engine: um caminho de conteúdo do modelo relativo à pasta Content do projeto.
- three.js, Bevy, raylib e Cocos Creator: cenas ou código para esses runtimes, com instruções ao lado da exportação.
- glTF / GLB: geometria portátil para ferramentas compatíveis; GLB é a forma binária. OBJ: geometria e materiais para ferramentas de modelagem.
- `.map`: **Exportar como** escolhe Valve 220, Standard, Quake 2 ou Quake 2 (Valve). A janela diz o que o dialeto escolhido perderia.

Leia os avisos depois de exportar, como texturas ausentes. **Resultado da verificação** os lista na janela.

# Conexões ao vivo: Godot, Unity, Defold e Flax
Uma conexão ao vivo envia edições não salvas do Heron Brush para o editor de uma engine enquanto você trabalha. Ela é de mão única: edições feitas na engine não voltam. Pode haver uma conexão ao vivo por janela de mapa. Pare-a antes de trocar de engine e reinicie-a para aplicar opções de exportação alteradas. Os controles ficam na janela Exportar, na página da engine, no cartão **Conexão ao vivo com**.

O indicador na barra de título mostra o estado da conexão; clique nele para pará-la. Fechar ou substituir o mapa interrompe a publicação. Depois de uma falha de importação, corrija o problema e inicie a conexão de novo.

## Godot
1. Exporte uma vez para um projeto Godot.
2. Clique em **Instalar complemento do Godot**.
3. Ative o Heron Brush Live em Project Settings → Plugins no Godot.
4. Clique em **Iniciar conexão ao vivo** e anexe o mapa, pelo painel do Godot, a uma cena 3D aberta.

O complemento substitui apenas o ramo que ele mesmo gerou e mantém o posicionamento do anexo e os outros nós. Salve a cena do Godot você mesmo e mantenha seus scripts e nós fora do ramo gerado.

## Unity
1. Exporte uma vez para dentro da pasta Assets do projeto Unity.
2. Clique em **Instalar complemento do Unity** e deixe o Unity compilá-lo.
3. Abra Window → Heron Brush → Live Link no Unity.
4. Inicie a conexão e anexe o mapa à cena ativa do Unity.

Mantenha a janela ao vivo do Unity aberta. As atualizações ficam pausadas no Play Mode e no Prefab Mode. Só o filho Generated do objeto do mapa é substituído. Não edite os arquivos gerados.

## Defold
1. Exporte uma vez para o seu projeto Defold e mantenha o projeto aberto no Defold.
2. Clique em **Iniciar conexão ao vivo**. Nenhum complemento é necessário.
3. Abra heron_brush_live/preview.collection no Defold ou faça referência a ela a partir da sua própria cena.

A conexão grava em uma pasta gerada separada e não mexe na exportação final nem no game.project. **Recarregar o jogo em execução** também atualiza um jogo iniciado pelo Defold; adicionar objetos ou alterar a colisão pode exigir reiniciar o nível. Falhas de conexão são repetidas automaticamente.

## Flax
Exporte para a pasta Content do projeto e clique em **Instalar complemento do Flax**. Deixe o Flax compilá-lo, inicie a conexão e então escolha Tools → Heron Brush: attach live map na cena aberta do Flax. A raiz gerada mantém o posicionamento e os outros objetos não são alterados. Salve a cena você mesmo. As atualizações ficam pausadas no Play Mode. Tools → Heron Brush: stop live map pausa a recepção.

# Sessões colaborativas ao vivo
Colaboração → Sessão ao vivo abre uma janela para hospedar uma sessão ou entrar na de outro usuário do Heron Brush. Isso compartilha a edição do mapa entre pessoas; é diferente de uma conexão ao vivo com uma engine.

## Hospedar e entrar
Preencha **Seu nome**. Para hospedar, escolha **Hospedar uma sessão**, clique em **Iniciar sessão** e envie aos convidados os dados de **Copiar dados de conexão**. Para entrar, escolha **Entrar em uma sessão**, cole os dados e clique em **Entrar**. Entrar substitui o mapa aberto pelo mapa do anfitrião, então salve o seu trabalho antes.

A versão padrão funciona na rede local, e **Porta da rede local** define a porta. Sessões pela internet (**Conectar pela internet**) exigem uma versão compilada com o recurso opcional de rede. Os participantes veem os marcadores de câmera e os nomes uns dos outros na vista 3D.

Fechar a janela mantém a sessão em andamento; o indicador na barra de título mostra o estado dela. **Encerrar sessão** ou **Sair da sessão** a encerra. Quando duas pessoas editam o mesmo objeto, a ordem do anfitrião decide a versão final, e desfazer também é uma edição compartilhada. Sempre que possível, trabalhem em áreas separadas e salvem pontos de controle.

# Exibição 3D e um segundo monitor
## Modos de exibição
A seção **Vista** da aba Mapa define o que as vistas desenham: Brushes, Entidades pontuais, Modelos de entidades, Nomes de classe das entidades, Contornos dos grupos, Patches, Materiais, Mostrar faces omitidas, Prévia de animações, Aramado, Arestas e Contornos de entidades brush. Aramado desenha as arestas dos triângulos, incluindo diagonais e arestas atrás das superfícies, sem materiais. É uma vista temporária de depuração e não afeta exportações nem renderizações.

Iluminação, sombras, oclusão ambiente, reflexos, bloom e suavização da vista 3D ficam em Preferências → Renderizador. São configurações de pré-visualização; as imagens finais usam as configurações da própria câmera de renderização.

## Uma janela 3D separada
Vista → Vista 3D em janela separada move a vista 3D para uma janela própria, que você pode colocar em outro monitor. Ela compartilha a câmera, o documento e as ferramentas, e as vistas planas ocupam o espaço na janela principal. O menu dela oferece Materiais, Vista sólida e Aramado; um submenu Vista com Mostrar faces omitidas, Prévia de animações, Arestas, Contornos dos grupos, Entidades pontuais, Modelos de entidades e Contornos de entidades brush; Focar na seleção; Frente, Lado e Topo, que giram a câmera em torno da seleção (ou de um ponto à frente da câmera); Posicionar câmera a partir da vista 3D; Tela cheia (`F11`); Renderizar imagem; e Voltar à janela principal. Fechar a janela acopla a vista de novo.

## Céu procedural
O céu procedural vem ativado por padrão na vista 3D e pode ser desligado em Preferências → Renderizador → Luz. As câmeras de renderização têm a própria opção **Céu procedural**, salva com o mapa. As cores do céu, do horizonte e do chão vêm das configurações de ambiente, e o sol acompanha a iluminação da cena. O céu não tem geometria e não é exportado.

# Câmeras de renderização e imagens PNG
Arquivo → Renderizar imagem abre a janela de renderização: a câmera e suas configurações à esquerda e uma prévia ao vivo à direita, que é redesenhada conforme você as altera. As câmeras são salvas no `.map` em uma camada Render cameras exclusiva do editor, que fica fora das exportações para engines.

## Posicionando a câmera
Enquadre a cena na vista 3D e pressione `Ctrl+Alt+0` (ou `Ctrl+Alt+Num0`), ou clique em **Posicionar câmera a partir da vista 3D**. Se nenhuma câmera estiver selecionada, a primeira salva é atualizada; se não houver nenhuma, uma é criada. Isso pode ser desfeito e mantém as configurações de saída e de iluminação. **Selecionar câmera no mapa**, **Carregar câmera do mapa** e **Mover a vista 3D para a câmera** funcionam no sentido inverso. **Enquadramento da câmera na vista 3D** mostra os limites da imagem de saída, e **Controlar a câmera pela vista 3D** deixa a navegação normal mover a câmera salva.

## Configurações
- **Posição da câmera**: X, Y, Z, guinada (yaw) e inclinação (pitch).
- **Saída e lente**: **Predefinições** (Full HD, Quadrado, Retrato), largura, altura, amostras, **Campo de visão vertical** e **Plano de corte distante**.
- **Iluminação**: Luz do dia, Pôr do sol ou Céu nublado, ou defina exposição, sol, cores do céu, do horizonte e do chão, **Céu procedural**, **Sombras**, **Oclusão ambiente** e **Bloom**.
- **Profundidade de campo**: **Ativar profundidade de campo**, distância de foco, faixa nítida e raio do desfoque. **Focar na seleção** foca no centro de um objeto selecionado à frente da câmera.

**Salvar câmera e configurações** guarda o formulário no mapa; salve o mapa para mantê-lo em disco.

## Renderizando
**Renderizar PNG** pergunta onde salvar e inicia uma renderização progressiva, que vai se completando amostra por amostra na prévia. Mais amostras ficam melhores, mas demoram mais. **Cancelar renderização** para sem gravar arquivo. O renderizador usa o pipeline de rasterização do editor, não path tracing. O PNG tem fundo opaco e as animações não são renderizadas. Imagens muito grandes podem travar o editor por um instante.

# Preferências
Arquivo → Preferências (`Ctrl+,`) abre as configurações. As alterações valem na hora e são salvas. A barra lateral lista Visão geral, Jogos, Vista, Cores, Mouse, Teclado e Renderizador. O campo de pesquisa no topo encontra uma configuração pelo nome ou pela descrição em qualquer página. Cada configuração alterada tem o próprio botão para restaurar o padrão, e **Restaurar padrões** redefine a página atual.

## As páginas
- **Visão geral**: as configurações mais comuns reunidas: Aparência (Opacidade dos painéis (%), Esquema de cores, Layout, Idioma), Edição, Vista e Manual.
- **Jogos**: o jogo, onde ficam os arquivos dele e o seu nome para colaboração. Um jogo define em que formato o mapa é gravado, quais definições de entidades carregar e onde ficam os materiais. As configurações de jogos são lidas da pasta `games` na pasta de configuração (`~/.config/heron-brush/games` no Linux e no macOS, `%LOCALAPPDATA%\heron-brush\games` no Windows) e da pasta `games` ao lado do aplicativo, onde os pacotes colocam a configuração incluída.
- **Vista**: Layout, Esquema de cores, Idioma, eixos do mundo, opacidade da grade, tamanho da célula na verificação de vazamentos e os bloqueios de mapeamento.
- **Cores**: as cores das vistas, da geometria e das alças.
- **Mouse**: velocidades e inversões de olhar, deslocar e mover, velocidade de voo, campo de visão, Vincular vistas 2D e tamanhos das alças.
- **Teclado**: o mapa de teclas, Heron Brush ou Blender.
- **Renderizador**: sombreamento realista, contorno da seleção, luz, sombras, oclusão ambiente, reflexos, bloom e superfícies.

## Idioma
**Idioma** oferece Usar o idioma do sistema e todos os idiomas traduzidos. A interface muda na hora. O manual abre no idioma da interface quando existe uma tradução dele e, caso contrário, em inglês.

# Plugins e MCP
## Plugins
Plugins estendem o editor com scripts Rhai. Arquivo → Plugins abre a janela Plugins. **Instalar um plugin** instala um arquivo `.rhai` e **Instalar uma pasta de plugin** instala uma pasta que contém `main.rhai`; os metadados opcionais em `plugin.toml` descrevem o plugin e suas configurações. Os plugins ficam agrupados em **Plugins ativos** e **Plugins inativos**, e **Ativar plugin** liga ou desliga um plugin; isso se mantém depois de reiniciar. **Recarregar plugins** lê os scripts do disco de novo e redefine os valores dos painéis deles.

Os plugins ativos também aparecem na seção **Plugins** da aba Mapa, com suas configurações, botões de ação (**Executar**) e saída. Uma ação atua na seleção ou, quando nada está selecionado, em tudo, e é um único passo de desfazer. O plugin Selection inspector, incluído no pacote, mostra contagens e ações de mover para cima e para baixo.

## MCP
Colaboração → Iniciar servidor para um agente (MCP) permite que um agente de IA ou outra ferramenta controle o editor aberto pelo Model Context Protocol. A janela mostra o endereço local com **Copiar**, e **Parar servidor** o encerra; enquanto ele está em execução, a barra de título mostra **MCP ativo**. A opção de linha de comando `--mcp` executa a mesma interface sem janela. Os clientes podem descobrir ferramentas para geometria, recursos, propriedades, animação, exportação e sessões. MCP e plugins do editor são interfaces diferentes.

# Linha de comando
O programa é `heron-brush`. Sem opções, ele abre a tela de boas-vindas.

## Abrindo arquivos
`heron-brush map.map [entities.fgd] [materials/]` abre um mapa, opcionalmente com definições de entidades e uma pasta de materiais. Um argumento vazio pula um deles, por exemplo `heron-brush map.map "" materials/`. Se um editor já estiver em execução, o mapa abre nele; `--new-instance` abre uma janela separada.

## Comandos
- `--help` ou `-h`: mostra o modo de uso. `--version` ou `-V`: mostra a versão.
- `--mcp`: serve MCP pela entrada e saída padrão, sem janela.
- `--merge-map <base> <ours> <theirs> [<output>]`: mesclagem de três vias de arquivos `.map` para o git. Por padrão, a saída é ours. Os conflitos são exibidos e geram um código de saída diferente de zero.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: transforma uma malha em brushes. Size é o comprimento do lado mais longo em unidades de mapa (padrão 256), snap é uma grade para os cantos (0 para nenhuma) e as palavras `solid` ou `layered` criam caixas alinhadas à grade ou placas convexas em vez de cunhas finas.
- `--render-preview <in.map> <out.png> [size]`: grava uma miniatura PNG de um mapa, com 512 pixels por padrão.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: executa um script de plugin sobre um mapa com tudo selecionado e grava o resultado (por padrão, de volta no arquivo de entrada). `key=value` responde às configurações do plugin.

# Solução de problemas
## Quando algo parece errado
- Material ausente: confira a pasta ou o pacote de materiais e os caminhos do projeto na exportação.
- Tamanho de modelo errado: compare com o marcador de jogador de 64 unidades e com a escala de exportação.
- Direção do gradiente errada: confira o ângulo do gradiente e a orientação da face.
- Cena vazia na engine: leia os avisos da exportação e siga as instruções de importação do destino.
- Conexão ao vivo esperando: confira se o complemento está instalado, se a cena está anexada e se as atualizações não estão pausadas.
- Só linhas, sem faces: desligue Aramado na seção Vista da aba Mapa ou escolha Materiais ou Vista sólida no menu da janela 3D.
- Objetos sumiram das vistas: `Ctrl+Shift+I` mostra os objetos ocultos; confira as camadas e os filtros da seção Vista.
- Uma tecla não faz nada: clique primeiro na vista para que ela tenha o foco do teclado e confira o mapa de teclas em Preferências → Teclado.
- Uma edição falhou: desfaça, simplifique a geometria e use **Verificar mapa** e **Verificar vazamentos** na seção **Problemas** da aba Mapa.
- Um mapa não abre pelo gerenciador de arquivos: ele pode ter sido entregue a um editor que já está em execução; procure essa janela.

## Configurações e backups
Preferências, mapas recentes, estrelas, configurações de exportação e plugins ficam na pasta de configuração: `~/.config/heron-brush` no Linux e no macOS e `%LOCALAPPDATA%\heron-brush` no Windows. Os backups automáticos ficam na pasta `autosave` ao lado de cada mapa.

## Relatando um bug
Inclua a versão do editor (Ajuda → Sobre), o seu sistema operacional, um mapa pequeno que mostre o problema e os passos exatos. O painel de saída (`Ctrl+4`) muitas vezes diz o que deu errado. Salve uma cópia antes de fazer experimentos.

## Usando este manual
Escolha um capítulo no sumário à esquerda. `Ctrl+F` mostra o sumário e leva ao campo de pesquisa; `Esc` limpa a pesquisa ou, quando ela está vazia, fecha o manual. **Sumário** oculta ou mostra a lista de capítulos e **Tamanho do texto** muda o tamanho da fonte; as duas opções são lembradas. A lista de idiomas no topo alterna entre as traduções disponíveis.
