IF OBJECT_ID(N'dbo.validation_results', N'U') IS NULL
BEGIN
    CREATE TABLE dbo.validation_results (
        id INT IDENTITY(1,1) PRIMARY KEY,
        user_name NVARCHAR(256) NOT NULL,
        value INT NOT NULL,
        valid BIT NOT NULL,
        validated_at DATETIME2 NOT NULL CONSTRAINT DF_validation_results_validated_at DEFAULT SYSUTCDATETIME()
    );
END;
