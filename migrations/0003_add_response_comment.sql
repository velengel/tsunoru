ALTER TABLE responses
ADD COLUMN respondent_comment TEXT
    CHECK (
        respondent_comment IS NULL
        OR (
            length(respondent_comment) BETWEEN 1 AND 500
            AND length(trim(respondent_comment)) > 0
            AND instr(respondent_comment, char(0)) = 0
        )
    );
