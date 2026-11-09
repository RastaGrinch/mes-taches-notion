# Configurer Notion

Cette application utilise une base Notion comme source distante. Depuis les versions récentes de l’API Notion, une base peut contenir plusieurs **sources de données**. L’application se connecte donc à un `data_source_id`, et non seulement à l’identifiant visuel de la base.

## 1. Créer la base

Dans Notion :

1. Créer une nouvelle page.
2. Choisir **Database** puis **New empty database**, ou utiliser `/database` dans une page existante.
3. Nommer la base, par exemple `Tâches`.
4. Renommer la propriété de titre initiale en `Tâche`.
5. Ajouter les propriétés du tableau ci-dessous en respectant exactement les majuscules, les accents, les espaces et la ponctuation.

| Nom exact | Type dans Notion | Valeurs recommandées | Utilisation |
| --- | --- | --- | --- |
| `Tâche` | Title | — | Titre de la tâche |
| `Done` | Checkbox | — | État terminé |
| `Statut` | Select | `A faire`, `En cours`, `Bloqué`, `Terminé`, `Abandonnée` | État de traitement |
| `Priorité` | Select | `P1 - Urgent & Important`, `P2 - Important`, `P3 - A faire`, `P4 - Plus tard` | Classement dans les vues |
| `Echéance` | Date | — | Date prévue |
| `Fait le` | Date | — | Date de réalisation |
| `Resp.` | Select | Libres, par exemple `Alice` | Responsable |
| `Source` | Select | Libres, par exemple `Mail` ou `Réunion` | Origine de la tâche |
| `description` | Text | — | Détails de la tâche |
| `Commentaire` | Text | — | Notes complémentaires |
| `Type` | Select | Libres | Propriété facultative |

Les dix premières propriétés sont obligatoires. `Type` est facultative : l’application la lit lorsqu’elle existe et ne l’écrit que si une valeur est définie.

## 2. Créer une connexion Notion

Dans Notion :

1. Ouvrir **Settings** puis **Connections**.
2. Activer **Developer Mode** si Notion le demande.
3. Ouvrir les jetons d’accès personnels ou créer une nouvelle connexion interne.
4. Donner un nom à la connexion, par exemple `Mes tâches local`.
5. Choisir le bon espace de travail.
6. Accorder les capacités de contenu suivantes :
   - **Read content** pour charger le schéma et les tâches ;
   - **Update content** pour modifier les tâches existantes ;
   - **Insert content** pour créer de nouvelles tâches.
7. Les informations utilisateur ne sont pas nécessaires.
8. Copier le jeton d’installation ou le jeton personnel. Il commence généralement par `ntn_`.

Documentation officielle : [créer une connexion avec l’API Notion](https://www.notion.com/help/create-integrations-with-the-notion-api) et [capacités des connexions](https://developers.notion.com/reference/capabilities).

## 3. Donner accès à la base

La connexion ne voit pas automatiquement toutes les pages de l’espace de travail.

1. Ouvrir la page qui contient la base `Tâches`.
2. Ouvrir le menu `•••` en haut à droite.
3. Choisir **More**, puis **Add connections** dans la section **Connections**.
4. Sélectionner la connexion créée précédemment.

Si la base affichée est une vue liée, ajouter la connexion à la page de la base d’origine. Les accès des sources liées suivent ceux de la source d’origine.

Documentation officielle : [ajouter et gérer les connexions Notion](https://www.notion.com/help/add-and-manage-connections-with-the-api).

## 4. Copier l’identifiant de la source de données

1. Ouvrir la base d’origine.
2. Ouvrir le menu de réglage de la base.
3. Choisir **Manage data sources**.
4. Ouvrir le menu `•••` de la source qui contient les tâches.
5. Choisir **Copy data source ID**.

Cette valeur ressemble à un UUID :

```text
11111111-2222-3333-4444-555555555555
```

Notion distingue désormais l’identifiant de la base et celui de la source de données. La procédure officielle est décrite dans le [guide de migration des sources de données](https://developers.notion.com/guides/get-started/upgrade-guide-2025-09-03#step-1-add-a-discovery-step-to-fetch-and-store-the-data_source_id).

## 5. Connecter l’application

1. Lancer **Mes tâches**.
2. Ouvrir **Paramètres**.
3. Coller l’identifiant dans **Identifiant de la source de données**.
4. Coller le jeton dans **Jeton personnel Notion**.
5. Régler si nécessaire le statut, le responsable et la source par défaut.
6. Choisir **Vérifier et connecter**.

Avant d’enregistrer la connexion, l’application :

- vérifie le jeton ;
- vérifie que la connexion a accès à la source ;
- compare les noms et types des propriétés ;
- effectue une requête de lecture limitée à une tâche.

Le jeton est ensuite conservé dans le coffre sécurisé du système. Il n’est pas enregistré dans SQLite.

## Résoudre les erreurs courantes

### Jeton invalide ou expiré

Créer ou recopier le jeton depuis **Settings → Connections**, puis le saisir à nouveau dans l’application.

### Accès interdit ou source introuvable

Vérifier que la connexion a été ajoutée à la page de la base d’origine et que l’identifiant copié est bien celui de la **source de données**.

### Schéma incompatible

Le message indique les propriétés absentes ou dont le type est incorrect. Corriger la base en respectant les noms exacts du tableau ci-dessus.

### La synchronisation est indisponible

Les modifications restent dans la file locale. Elles seront envoyées à la prochaine synchronisation réussie.

