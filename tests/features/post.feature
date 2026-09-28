Feature: Posts
    API de gestion des posts

    Background:
        Given la base de données est réinitialisée


    # ============================================================================
    # POST /api/posts
    # ============================================================================

    Scenario: Créer un post avec succès
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        When je crée un post avec:
            | title     | Mon premier post |
            | content   | Contenu du post  |
            | published | false            |
        Then le statut de la réponse est 201
        And la réponse contient:
            | title   | Mon premier post |
            | content | Contenu du post  |
        And la réponse contient un champ "id"
        And la réponse contient un champ "user_id"
        And le mot de passe n'apparaît pas dans la réponse

    Scenario: Créer un post sans "published" le met à false par défaut
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        When je crée un post avec:
            | title   | Post sans published |
            | content | Contenu             |
        Then le statut de la réponse est 201
        And le champ "published" de la réponse vaut "false"

    Scenario: Créer un post avec un titre vide échoue
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        When je crée un post avec:
            | title   |                |
            | content | Contenu valide |
        Then le statut de la réponse est 400

    Scenario: Créer un post sans authentification échoue
        When je crée un post sans token avec:
            | title   | Post sans token |
            | content | Contenu         |
        Then le statut de la réponse est 401


    # ============================================================================
    # GET /api/posts
    # ============================================================================

    Scenario: Lister les posts quand il n'y en a aucun
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        When je liste mes posts
        Then le statut de la réponse est 200
        And la réponse est une liste vide

    Scenario: Lister les posts ne retourne que les siens
        Given je suis connecté en tant que "user_a@test.com" avec le mot de passe "securepass"
        And je crée un post avec:
            | title   | Post A1 |
            | content | Contenu |
        And je crée un post avec:
            | title   | Post A2 |
            | content | Contenu |
        And je suis connecté en tant que "user_b@test.com" avec le mot de passe "securepass"
        And je crée un post avec:
            | title   | Post B1 |
            | content | Contenu |
        When je liste les posts de "user_a@test.com"
        Then le statut de la réponse est 200
        And la réponse contient exactement 2 posts
        And aucun post de la liste n'a le titre "Post B1"


    # ============================================================================
    # GET /api/posts/:id
    # ============================================================================

    Scenario: Récupérer un post par son id avec succès
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        And j'ai créé un post avec:
            | title   | Mon post |
            | content | Contenu  |
        When je récupère le post par son id
        Then le statut de la réponse est 200
        And la réponse contient:
            | title | Mon post |

    Scenario: Récupérer un post inexistant retourne 404
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        When je récupère le post avec l'id "00000000-0000-0000-0000-000000000000"
        Then le statut de la réponse est 404


    # ============================================================================
    # PUT /api/posts/:id
    # ============================================================================

    Scenario: Modifier son propre post avec succès
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        And j'ai créé un post avec:
            | title   | Titre original   |
            | content | Contenu original |
        When je modifie le post avec:
            | title     | Titre modifié |
            | published | true          |
        Then le statut de la réponse est 200
        And la réponse contient:
            | title   | Titre modifié    |
            | content | Contenu original |
        And le champ "published" de la réponse vaut "true"

    Scenario: Modifier le post d'un autre utilisateur est interdit
        Given je suis connecté en tant que "owner@test.com" avec le mot de passe "securepass"
        And j'ai créé un post avec:
            | title   | Post de A |
            | content | Contenu   |
        And je suis connecté en tant que "hacker@test.com" avec le mot de passe "securepass"
        When je modifie le post avec:
            | title | Post hacké |
        Then le statut de la réponse est 403

    Scenario: Modifier un post inexistant retourne 404
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        When je modifie le post "00000000-0000-0000-0000-000000000000" avec:
            | title | Peu importe |
        Then le statut de la réponse est 404


    # ============================================================================
    # DELETE /api/posts/:id
    # ============================================================================

    Scenario: Supprimer son propre post avec succès
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        And j'ai créé un post avec:
            | title   | Post à supprimer |
            | content | Contenu          |
        When je supprime le post
        Then le statut de la réponse est 204
        And le post n'est plus accessible

    Scenario: Supprimer le post d'un autre utilisateur est interdit
        Given je suis connecté en tant que "owner@test.com" avec le mot de passe "securepass"
        And j'ai créé un post avec:
            | title   | Post de A |
            | content | Contenu   |
        And je suis connecté en tant que "hacker@test.com" avec le mot de passe "securepass"
        When je supprime le post
        Then le statut de la réponse est 403


    # ============================================================================
    # Cascade — DELETE user supprime ses posts
    # ============================================================================

    Scenario: Supprimer un utilisateur supprime aussi ses posts en cascade
        Given je suis connecté en tant que "author@test.com" avec le mot de passe "securepass"
        And j'ai créé un post avec:
            | title   | Post orphelin |
            | content | Contenu       |
        When je supprime mon compte
        And je suis connecté en tant que "other@test.com" avec le mot de passe "securepass"
        And je récupère le post par son id
        Then le statut de la réponse est 404
