**Languages:** [English](README.md) | [Türkçe](README.tr.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Italiano](README.it.md) | [Русский](README.ru.md)

<p align="center">
  <img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="220" height="220" alt="Icône PULS"/>
</p>

# PULS

**Un outil unifié de surveillance et d'administration système pour Linux**

PULS associe une surveillance des ressources haute fidélité à des fonctionnalités natives d'administration système sous Linux. Il vous permet de surveiller la télémétrie matérielle, de gérer les services systemd, d'analyser les journaux système, d'examiner les arbres de processus hiérarchiques, d'émettre des signaux POSIX, de modifier en toute sécurité les configurations de démarrage et de suivre les conteneurs directement depuis une interface utilisateur de terminal interactive (TUI).

![Capture d'écran PULS](https://raw.githubusercontent.com/word-sys/puls/main/screenshots/screenshot.png)

---

## Architecture et Principes de Conception

PULS est développé en Rust avec `ratatui` et `crossterm` pour le rendu TUI, communiquant directement avec le noyau Linux et les interfaces système natives :

*   **Zéro Nouvelle Dépendance** : Élimine les dépendances externes superflues en s'appuyant sur la bibliothèque standard, des appels FFI Unix personnalisés et les systèmes de fichiers virtuels du noyau (`/proc`, `/sys`).
*   **Télémétrie Matérielle Approfondie** : Analyse native de la topologie CPU (caches L1/L2/L3, affinités des nœuds NUMA, gouverneurs de fréquence), de la répartition des pages mémoire, des inodes de systèmes de fichiers (`statvfs`) et de la puissance de la batterie (`/sys/class/power_supply/`).
*   **Support Multi-Constructeurs GPU** : Analyseurs de pilotes sysfs/DRM et hwmon pour les cartes graphiques AMD et Intel, combinés aux interfaces NVML/`nvidia-smi` pour NVIDIA. Surveillance de l'utilisation de la VRAM, des générations et largeurs de liens PCIe, des vitesses de ventilation (tr/min), de la consommation par rapport aux limites d'alimentation et des alertes de limitation thermique matérielle.
*   **Éditeur de Démarrage Transactionnel** : Modifiez `/etc/default/grub` en mémoire. Visualisez un diff complet (`u`) avant toute écriture sur le disque, avec création automatique de sauvegardes horodatées.
*   **Contrôle Terminal Résilient** : Capture des événements souris (`EnableMouseCapture`) pour le changement d'onglet et le défilement fluide, protégée par un gestionnaire de panique restaurant l'état normal du terminal en toute circonstance.

---

## Fonctionnalités

### 1. Surveillance des Ressources et du Matériel
*   **CPU et Architecture NUMA** : Graphiques d'utilisation par cœur en temps réel, disposition adaptative en grille pour les processeurs à grand nombre de cœurs (jusqu'à 128+ cœurs), métriques de cache L1/L2/L3, gouverneurs de fréquence et affinités de cœurs/mémoire par nœud NUMA.
*   **Mémoire et Swap** : Décomposition visuelle de la mémoire totale, utilisée, libre, disponible, en cache et du swap. Prise en charge des unités de température en Celsius et Fahrenheit.
*   **Stockage et Inodes** : Débits de lecture et d'écriture par partition, espace utilisé, options de montage et pourcentages d'inodes alloués (`statvfs`).
*   **Réseau et Sockets** : Débits d'envoi (`^`) et de réception (`v`) en temps réel. Inspecteur de sockets TCP/UDP actifs avec affichage des adresses IP locales/distantes, des états de connexion (ESTABLISHED, LISTEN, etc.) et des PID propriétaires.
*   **GPU NVIDIA, AMD et Intel** : Surveillance multi-cartes affichant la charge de calcul, l'utilisation de la VRAM, les fréquences, les températures, les vitesses des ventilateurs (tr/min), le lien PCIe, la limite de puissance et le statut de limitation thermique.
*   **Alimentation et Batterie** : État de santé de la batterie, puissance de charge (Watts), cycles, jauges de capacité et état de l'adaptateur secteur.

### 2. Gestion des Processus et des Conteneurs
*   **Arbre Hiérarchique des Processus** : Basculez entre la liste plate et la vue arborescente parent-enfant (`t`) avec des glyphes d'arbre clairs (`├─`, `└─`).
*   **Sélecteur de Signaux POSIX** : Envoyez directement des signaux Unix aux processus via un sélecteur interactif (`k` ou `F9`) prenant en charge `SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT` et `SIGINT`.
*   **Ajustement de Priorité (Renice)** : Modifiez dynamiquement la priorité d'ordonnancement (`[` pour augmenter la priorité / diminuer la valeur nice, `]` pour diminuer la priorité / augmenter la valeur nice).
*   **Inspecteur Détaillé de Processus** : Appuyez sur `Entrée` sur n'importe quel processus pour examiner le nombre de descripteurs de fichiers ouverts (`/proc/[pid]/fd`), de sockets actifs, de threads et l'utilisation CPU par thread (`/proc/[pid]/task`).
*   **Recherche Interactive** : Appuyez sur `/` dans l'onglet Processus pour filtrer en temps réel par nom ; `Échap` efface le filtre.
*   **Conteneurs Docker et Podman** : Se connecte au démon Docker ou aux sockets Podman utilisateur (`/run/user/$UID/podman/podman.sock`). Démarrer (`s`), Arrêter (`x`), Redémarrer (`r`), Suspendre (`p`) et visionner les journaux en direct (`Entrée` ou `l`).

### 3. Sous-Système d'Administration des Services
Intégration directe avec `systemd` (`systemctl`) :
*   **Contrôle d'État** : Démarrer (`s`), Arrêter (`x`) et Redémarrer (`r`) les services.
*   **Persistance au Démarrage** : Activer (`e`) ou Désactiver (`d`) des services au démarrage du système.
*   **Définition du Service** : Consultez la configuration complète de l'unité systemd, les dépendances et les statuts de validation.
*   **Visualiseur de Journaux** : Appuyez sur `g` pour afficher les 50 dernières entrées `journald` pour le service sélectionné.
*   **Minuteurs Programmés** : Vue d'ensemble des minuteurs systemd actifs (`systemctl list-timers`) avec les prochains déclenchements.

### 4. Administration Système et Diagnostics
*   **Diagnostics Système** : Panneau d'anomalies sur le tableau de bord signalant les surchauffes CPU, la pression mémoire et le stockage critique.
*   **Sessions et Redémarrage** : Sessions utilisateur actives (`who` / `/var/run/utmp`) et indicateur de redémarrage système requis (`/var/run/reboot-required`).
*   **Journaux Journald Centralisés** : Filtrez les journaux par priorité (Erreur/Avertissement), unité ou session de démarrage. Requêtes optimisées avec `--boot=0`.

### 5. Personnalisation et Internationalisation
*   **7 Langues Disponibles** : Prise en charge intégrale du français, de l'anglais (par défaut), du turc, de l'allemand, de l'espagnol, de l'italien et du russe. Détection automatique via `LANG`/`LC_ALL` ou changement dynamique (`L` / `l`).
*   **Dialogue de Paramètres (`F2` / `Shift+S`)** : Modifiez la langue, le thème de couleur, l'onglet par défaut au démarrage, l'unité de température (Celsius / Fahrenheit), la fréquence de rafraîchissement (500ms à 5000ms) et le mode d'affichage des processus. Enregistré automatiquement dans `~/.config/puls/config.ini`.
*   **6 Thèmes de Couleurs** : Défaut, Bleu Foncé, Clair, Dracula, Solarized Sombre et Haut Contraste.
*   **Support Souris** : Clic pour la sélection des onglets et le bouton Paramètres, molette pour le défilement fluide des tableaux et journaux.

---

## Raccourcis Clavier et Contrôles

### Navigation Globale
| Touche | Action |
| :--- | :--- |
| `q` / `Échap` | Quitter PULS (ou fermer la boîte de dialogue active) |
| `Tab` / `1`..`9`, `0`, `-`, `=` | Changer d'onglet (0: Tableau de bord, 1: CPU, 2: Mémoire, 3: Processus, 4: Disques, 5: Réseau, 6: GPU, 7: Capteurs, 8: Services, 9: Journaux, 10: GRUB, 11: Conteneurs, 12: Infos Système) |
| `Clic Gauche` | Cliquer sur un onglet d'en-tête ou sur le bouton des Paramètres |
| `F2` / `Shift+S` / `s` | Ouvrir / Fermer la boîte de dialogue des Paramètres |
| `L` / `l` | Faire défiler la langue (FR -> DE -> ES -> IT -> RU -> EN -> TR) |
| `t` / `T` | Changer de thème de couleur (en dehors de l'onglet Processus) |
| `p` | Mettre en pause / reprendre la collecte des métriques |
| `?` | Ouvrir / Fermer l'aide et la liste des raccourcis |
| `Haut` / `Bas` / `j` / `k` / `Molette` | Naviguer dans les listes et faire défiler les fenêtres modales |

### Onglet Processus (Onglet 3)
| Touche | Action |
| :--- | :--- |
| `t` | Basculer entre la liste plate et la vue arborescente hiérarchique |
| `k` / `F9` | Ouvrir le sélecteur de signaux POSIX (`SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT`, `SIGINT`) |
| `[` | Augmenter la priorité du processus (renice -1, priorité plus élevée) |
| `]` | Diminuer la priorité du processus (renice +1, priorité plus basse) |
| `Entrée` | Ouvrir l'inspecteur détaillé (descripteurs, sockets, threads) |
| `/` | Filtrer les processus par nom |

### Onglet Services (Onglet 8)
| Touche | Action |
| :--- | :--- |
| `s` | Démarrer le service sélectionné |
| `x` | Arrêter le service sélectionné |
| `r` | Redémarrer le service sélectionné |
| `e` | Activer le service au démarrage |
| `d` | Désactiver le service au démarrage |
| `g` | Afficher les 50 dernières lignes de journaux journald |

### Onglet Conteneurs (Onglet 11)
| Touche | Action |
| :--- | :--- |
| `s` | Démarrer le conteneur |
| `x` | Arrêter le conteneur |
| `r` | Redémarrer le conteneur |
| `p` | Suspendre / Reprendre le conteneur |
| `Entrée` / `l` | Ouvrir la fenêtre des journaux en direct |

### Onglet Configuration GRUB (Onglet 10)
| Touche | Action |
| :--- | :--- |
| `Entrée` | Modifier le paramètre de configuration sélectionné |
| `u` | Ouvrir le diff de révision transactionnelle avant enregistrement |

---

## Installation

### Binaire Statique (Portable)
L'installation recommandée utilise le binaire autonome lié à MUSL, ne nécessitant aucune dépendance d'exécution ni version spécifique de la glibc. Disponible pour AMD64 (`x86_64`) et ARM64 (`aarch64`) :

```bash
# Pour x86_64 (AMD64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-x86_64-linux-musl.tar.gz
tar -xzf puls-0.9.4-x86_64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls

# Pour aarch64 (ARM64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-aarch64-linux-musl.tar.gz
tar -xzf puls-0.9.4-aarch64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls
```

### Paquet Debian / Ubuntu (.deb)
Installez le paquet `.deb` officiel sur Debian, Ubuntu, Linux Mint ou Pop!_OS :

```bash
# Télécharger et installer le paquet AMD64
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_amd64.deb
sudo apt install ./puls_0.9.4_amd64.deb

# Pour systèmes ARM64
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_arm64.deb
sudo apt install ./puls_0.9.4_arm64.deb
```

Pour corriger d'éventuelles dépendances :
```bash
sudo apt --fix-broken install
```

### Compilation depuis les Sources
Pour compiler PULS localement :

1. **Prérequis** :
   * Chaîne d'outils Rust (1.70+) : `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   * Outils de compilation musl : `sudo apt install musl-tools` (Debian/Ubuntu) ou `sudo dnf install musl-gcc` (Fedora) ou `sudo pacman -S musl` (Arch)
   * Ajouter la cible musl : `rustup target add x86_64-unknown-linux-musl`

2. **Compilation** :
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
   Le binaire est généré dans `target/x86_64-unknown-linux-musl/release/puls`.

---

## Modes d'Utilisation

PULS adapte ses privilèges et fonctionnalités selon le mode de lancement :

| Commande | Mode Opérationnel |
| :--- | :--- |
| `puls` | **Mode Utilisateur** : Surveillance complète des processus, CPU, mémoire, disques, réseau, GPU et conteneurs. |
| `sudo puls` | **Mode Administrateur** : Accès complet au contrôle des services systemd, aux journaux et à l'édition de GRUB. |
| `puls --safe` | **Mode Sécurisé** : Désactive toutes les opérations d'écriture pour éviter toute modification accidentelle. |
| `puls --telemetry` | **Mode Télémétrie** : Affiche les métriques de temps de démarrage détaillées sur la sortie standard. |

---

## Licence

PULS est distribué sous la licence [GNU General Public License v3.0](LICENSE).
