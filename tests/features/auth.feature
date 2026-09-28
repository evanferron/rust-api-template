Feature: Auth
    API authentication workflows

    Background:
        Given la base de données est réinitialisée


    # ============================================================================
    # POST /api/auth/register
    # ============================================================================

    Scenario: Register successfully with valid credentials
        When je m'enregistre avec:
            | email      | register@test.com |
            | password   | securepass        |
            | first_name | Evan              |
            | last_name  | Ferron            |
        Then le statut de la réponse est 201
        And la réponse contient:
            | email      | register@test.com |
            | first_name | Evan              |
            | last_name  | Ferron            |
        And le hash du mot de passe n'apparaît pas dans la réponse

    Scenario: Register fails with invalid email
        When je m'enregistre avec:
            | email      | not-an-email |
            | password   | securepass   |
            | first_name | Evan         |
            | last_name  | Ferron       |
        Then le statut de la réponse est 400

    Scenario: Register fails with password too short
        When je m'enregistre avec:
            | email      | valid@test.com |
            | password   | short          |
            | first_name | Evan           |
            | last_name  | Ferron         |
        Then le statut de la réponse est 400

    Scenario: Register fails with duplicate email
        When je m'enregistre avec:
            | email      | duplicate@test.com |
            | password   | securepass         |
            | first_name | Evan               |
            | last_name  | Ferron             |
        And je m'enregistre à nouveau avec:
            | email      | duplicate@test.com |
            | password   | securepass         |
            | first_name | Evan               |
            | last_name  | Ferron             |
        Then le statut de la réponse est 409


    # ============================================================================
    # POST /api/auth/login
    # ============================================================================

    Scenario: Login successfully with valid credentials
        Given je suis enregistré avec:
            | email    | login@test.com |
            | password | securepass     |
        When je me connecte avec:
            | email    | login@test.com |
            | password | securepass     |
        Then le statut de la réponse est 200
        And la réponse contient un access_token

    Scenario: Login fails with wrong password
        Given je suis enregistré avec:
            | email    | user@test.com |
            | password | correctpass   |
        When je me connecte avec:
            | email    | user@test.com |
            | password | wrongpass     |
        Then le statut de la réponse est 401

    Scenario: Login fails with unknown email
        When je me connecte avec:
            | email    | ghost@test.com |
            | password | anypass        |
        Then le statut de la réponse est 401


    # ============================================================================
    # POST /api/auth/refresh
    # ============================================================================

    Scenario: Refresh token succeeds with valid refresh cookie
        Given je suis enregistré et connecté avec:
            | email    | refresh@test.com |
            | password | securepass       |
        When j'appelle l'endpoint refresh avec le refresh_token cookie
        Then le statut de la réponse est 200
        And la réponse contient un nouvel access_token

    Scenario: Refresh token fails without refresh cookie
        When j'appelle l'endpoint refresh sans cookie
        Then le statut de la réponse est 401

