Feature: Users
    API de gestion des utilisateurs

    Background:
        Given la base de données est réinitialisée


    # ============================================================================
    # GET /api/users
    # ============================================================================

    Scenario: Lister les utilisateurs en étant authentifié
        Given je suis connecté en tant que "user@test.com" avec le mot de passe "securepass"
        When je liste les utilisateurs
        Then le statut de la réponse est 200
        And la réponse est une liste de 1 utilisateur

    Scenario: Lister les utilisateurs sans authentification échoue
        When je liste les utilisateurs sans token
        Then le statut de la réponse est 401


    # ============================================================================
    # GET /api/users/:id
    # ============================================================================

    Scenario: Récupérer un utilisateur par son id avec succès
        Given je suis connecté en tant que "getbyid@test.com" avec le mot de passe "securepass"
        When je récupère mon profil par son id
        Then le statut de la réponse est 200
        And la réponse contient:
            | email | getbyid@test.com |

    Scenario: Récupérer un utilisateur inexistant retourne 404
        Given je suis connecté en tant que "user@test.com" avec le mot de passe "securepass"
        When je récupère l'utilisateur avec l'id "00000000-0000-0000-0000-000000000000"
        Then le statut de la réponse est 404


    # ============================================================================
    # PUT /api/users/:id
    # ============================================================================

    Scenario: Modifier son propre profil avec succès
        Given je suis connecté en tant que "update@test.com" avec le mot de passe "securepass"
        When je modifie mon profil avec:
            | first_name | Updated |
        Then le statut de la réponse est 200
        And la réponse contient:
            | first_name | Updated |

    Scenario: Modifier le profil d'un autre utilisateur est interdit
        Given je suis connecté en tant que "user_a@test.com" avec le mot de passe "securepass"
        And je suis connecté en tant que "user_b@test.com" avec le mot de passe "securepass"
        When je modifie le profil de "user_a@test.com" avec:
            | first_name | Hacked |
        Then le statut de la réponse est 403


    # ============================================================================
    # DELETE /api/users/:id
    # ============================================================================

    Scenario: Supprimer son propre compte avec succès
        Given je suis connecté en tant que "delete@test.com" avec le mot de passe "securepass"
        When je supprime mon compte
        Then le statut de la réponse est 204

    Scenario: Supprimer le compte d'un autre utilisateur est interdit
        Given je suis connecté en tant que "victim@test.com" avec le mot de passe "securepass"
        And je suis connecté en tant que "attacker@test.com" avec le mot de passe "securepass"
        When je supprime le compte de "victim@test.com"
        Then le statut de la réponse est 403
