Feature: Auth
    API d'authentification : inscription, connexion, renouvellement du token

    Background:
        Given la base de données est réinitialisée


    # ============================================================================
    # POST /api/auth/register
    # ============================================================================

    Scenario: S'inscrire avec succès
        When je m'enregistre avec:
            | email      | register@test.com |
            | password   | securepass        |
            | first_name | Jane              |
            | last_name  | Doe               |
        Then le statut de la réponse est 201
        And la réponse contient:
            | email      | register@test.com |
            | first_name | Jane              |
            | last_name  | Doe               |
        And le hash du mot de passe n'apparaît pas dans la réponse

    Scenario: S'inscrire avec un email invalide échoue
        When je m'enregistre avec:
            | email      | not-an-email |
            | password   | securepass   |
            | first_name | Jane         |
            | last_name  | Doe          |
        Then le statut de la réponse est 422
        And la réponse contient:
            | code | VALIDATION_FAILED |
        And la réponse signale une erreur sur le champ "email"

    Scenario: S'inscrire avec un mot de passe trop court échoue
        When je m'enregistre avec:
            | email      | valid@test.com |
            | password   | short          |
            | first_name | Jane           |
            | last_name  | Doe            |
        Then le statut de la réponse est 422
        And la réponse contient:
            | code | VALIDATION_FAILED |
        And la réponse signale une erreur sur le champ "password"

    Scenario: S'inscrire avec un email déjà utilisé échoue
        When je m'enregistre avec:
            | email      | duplicate@test.com |
            | password   | securepass         |
            | first_name | Jane               |
            | last_name  | Doe                |
        And je m'enregistre à nouveau avec:
            | email      | duplicate@test.com |
            | password   | securepass         |
            | first_name | Jane               |
            | last_name  | Doe                |
        Then le statut de la réponse est 409
        And la réponse contient:
            | code | EMAIL_ALREADY_USED |


    # ============================================================================
    # POST /api/auth/login
    # ============================================================================

    Scenario: Se connecter avec succès
        Given je suis enregistré avec:
            | email    | login@test.com |
            | password | securepass     |
        When je me connecte avec:
            | email    | login@test.com |
            | password | securepass     |
        Then le statut de la réponse est 200
        And la réponse contient un access_token

    Scenario: Se connecter avec un mauvais mot de passe échoue
        Given je suis enregistré avec:
            | email    | user@test.com |
            | password | correctpass   |
        When je me connecte avec:
            | email    | user@test.com |
            | password | wrongpass     |
        Then le statut de la réponse est 401
        And la réponse contient:
            | code | INVALID_CREDENTIALS |

    Scenario: Se connecter avec un email inconnu échoue
        When je me connecte avec:
            | email    | ghost@test.com |
            | password | anypass        |
        Then le statut de la réponse est 401
        And la réponse contient:
            | code | INVALID_CREDENTIALS |


    # ============================================================================
    # POST /api/auth/refresh
    # ============================================================================

    Scenario: Renouveler le token avec le cookie de refresh
        Given je suis enregistré et connecté avec:
            | email    | refresh@test.com |
            | password | securepass       |
        When j'appelle l'endpoint refresh avec le refresh_token cookie
        Then le statut de la réponse est 200
        And la réponse contient un nouvel access_token

    Scenario: Renouveler le token sans cookie de refresh échoue
        When j'appelle l'endpoint refresh sans cookie
        Then le statut de la réponse est 401
        And la réponse contient:
            | code | UNAUTHORIZED |

