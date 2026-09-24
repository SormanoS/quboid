# Registro delle modifiche

Cosa è cambiato in ogni versione di Quboid, per chi lo usa. La versione inglese
è [`CHANGELOG.md`](CHANGELOG.md); i due file vanno aggiornati insieme.

## [0.1.3] - 2026-09-24

### Aggiunto

- Annulla, con `Ctrl+Alt+Z`, riporta indietro l'ultimo posizionamento di una
  finestra un passo alla volta, compresa una massimizzazione.
- `Ctrl+Alt+S` mostra tutte le azioni e i tasti che le eseguono, e si apre
  anche quando Quboid è nell'area di notifica.
- Le finestre tornano sullo schermo in cui erano dopo aver collegato o
  scollegato un dock o cambiato risoluzione. L'impostazione è attiva di
  default.
- Avviare Quboid quando è già in esecuzione apre la finestra delle impostazioni,
  invece di avviare una seconda copia che non avrebbe nessuna scorciatoia.
- L'icona nell'area di notifica indica quante scorciatoie sono occupate da un
  altro programma.

### Modificato

- Lo spazio tra le finestre ha la stessa dimensione su ogni monitor, qualunque
  sia la sua scala.
- Una configurazione di una versione precedente riceve le scorciatoie
  predefinite delle nuove azioni, senza sostituire le combinazioni che usi già.
- Il programma di installazione chiude Quboid in esecuzione prima di
  aggiornarlo e lo riavvia dopo, così la nuova versione ha tutte le scorciatoie.

### Corretto

- I titoli dei gruppi nella pagina delle azioni sono leggibili con il tema
  chiaro.

## [0.1.2] - 2026-09-11

### Modificato

- Prima versione pubblicata su GitHub, come programma di installazione e come
  archivio portable.
- Quboid non richiede più Visual C++ Redistributable.

## [0.1.1]

Mai distribuita: il suo tag è stato ritirato prima che il rilascio fosse
completato. Le sue modifiche sono in 0.1.2.

## [0.1.0] - 2026-09-09

### Aggiunto

- Prima versione, sul Microsoft Store.
- Metà, terzi, quarti, centratura, ridimensionamento e spostamento graduale con
  scorciatoie globali, un'azione per scorciatoia.
- Spostamento tra monitor, anche con coordinate negative e scale diverse.
- Aggancio trascinando ai bordi e agli angoli dello schermo, con l'anteprima
  dell'area.
- Uno spazio configurabile tra le finestre.
- Interfaccia in italiano e in inglese, scelta dalla lingua di Windows al primo
  avvio.
- Modalità area di notifica e avvio all'accesso.
- Configurazione JSON con importazione, esportazione e modalità portable.
