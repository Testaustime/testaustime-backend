# Contributing

## Document any api changes
Api documentation is in [/docs/APISPEC.md](/docs/APISPEC.md). 

Some required points of docing API changes:

1. You must document any new endpoints you add, any endpoints you remove or any old endpoint that you change
2. You have to doc new routes in "General Info" section  (`/auth/, /users/, /activity/, /friends/, /friends/, /leaderboards/` and etc.)
3. You have to doc new endpoint in "Endpoints" section of the existing route. Add the method and short description by using existing format of tables in doc. The table of endpoints consists of links to each endpoint. Each endpoint in turn links to it's route
5. For the description of the endpoint pls use this sample:

<details>
  <summary>Sample of the endpoint description</summary>

  #### <a name="link_on_this_method"></a>  [1. METHOD /route/...](#link_on_existing_route)
  Short description to 1-3 sentences. 

  >For docing params and error examples of your endpoint please use [HTML details element](https://gist.github.com/scmx/eca72d44afee0113ceb0349dd54a84a2)

<details>
  <summary>Header params (if existing):</summary>

| Name | Value | 
| --- | --- | 
| Name of header param | Value |
</details> 

<details>
  <summary>Path params (if existing):</summary>

| Path param |  Description | 
| --- | --- | 
| Name of path param | Description of path param |   
</details> 

<details>
  <summary>Query string params (if existing):</summary>

| Param |  Type | Required | Description |
| --- | --- | --- | --- |
| Query-string param| Type of param (int/string) | Yes/No | Description of param |
</details> 

  **Sample request** 
  ```curl
  1. Please use curl-requests for sample sections, including main params (Header params, body params, path params, query params). 
  2. For example you can export request from Postman in curl format.
  3. Try to use realistic values in sample, but not real. Don't show any auth info of your user in doc
  ```
  **Sample response**
  ```curl
  1. There is no need to show any response headers in response. 
  2. In case response contains a body, please display it 
  3. In case response contains only http code status, please display it
  ```
  Optionally you can add some error examples, if there are any not obvious unseccsful use cases with your endpoints

<details>
  <summary>Error examples:</summary>

  | Error | Error code | Body | 
  | --- | --- | --- | 
  | Error description| Error code status | Body of error response |
</details> 

</details>

## Format code with rustfmt
This can be done by doing `cargo fmt` in the project directory
