use super::{
    aux::*,
    blob::Blob,
    git_object::{GitObject, GitObjectTrait},
    mode::Mode,
};
use crate::{
    commands::{command_errors::CommandError, objects_database},
    logger::Logger,
};
use std::{
    collections::HashMap,
    fmt,
    io::{Read, Write}, clone,
};

#[derive(Clone)]
pub struct Tree {
    path: String,
    objects: HashMap<String, GitObject>,
}

impl Tree {
    pub fn add_path_tree(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        Ok(self.add_path(vector_path, current_depth, hash)?)
    }

    /// Crea un Tree a partir de su ruta y los objetos a los que referencia. Si la ruta no existe,
    /// devuelve Error.
    pub fn new(path: String) -> Tree {
        Tree {
            path: path.clone(),
            objects: HashMap::new(),
        }
    }

    /// Crea un Blob a partir de su hash y lo añade al Tree.
    pub fn add_blob(&mut self, path_name: &String, hash: &String) -> Result<(), CommandError> {
        let blob = Blob::new_from_hash(hash.clone(), path_name.clone())?;
        _ = self.objects.insert(path_name.to_string(), Box::new(blob));
        Ok(())
    }

    // fn get_tree(&self, path: &str) -> &mut Tree {
    //     let object = self.objects.get(path).unwrap();
    //     let tree = object.as_mut_tree().unwrap();
    //     tree
    // }

    // To fix this error, you need to ensure that you are working with a mutable reference to tree instead of a & reference.
    // One way to do this is to change the type of tree to &mut Box<dyn Object>.

    // pub fn add_tree(
    //     &mut self,
    //     path_str: &String,
    //     vector_path: Vec<&str>,
    //     current_depth: usize,
    //     hash: &String,
    // ) -> Result<(), CommandError> {
    //     let path_name = get_name(path_str)?;

    //     if !self.objects.contains_key(&path_name) {
    //         let tree2 = Tree::new(path_str.to_owned());
    //         self.objects.insert(path_str.clone(), Box::new(tree2));
    //     }

    //     let tree = self.objects.get(&path_name).unwrap();

    //     tree.add_path(vector_path, current_depth + 1, hash)?;
    //     Ok(())
    // }

    pub fn add_tree(
        &mut self,
        current_path: &String,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        let tree_name = get_name(current_path)?;

        if !self.objects.contains_key(&tree_name) {
            let tree2 = Tree::new(current_path.to_owned());
            self.objects.insert(tree_name.clone(), Box::new(tree2));
        }

        let tree = self.objects.get_mut(&tree_name).unwrap();

        tree.add_path(vector_path, current_depth + 1, hash)?;
        Ok(())
    }

    fn get_data(&self) -> Result<Vec<u8>, CommandError> {
        let header = format!("1 {}\0", self.size()?);
        let content = self.content()?;
        Ok([header.as_bytes(), content.as_slice()].concat())
    }

    fn get_mode(&self) -> Result<Mode, CommandError> {
        Ok(Mode::get_mode(self.path.clone())?)
    }

    pub fn read_from(
        stream: &mut dyn Read,
        len: usize,
        path: &str,
        hash: &str,
        logger: &mut Logger,
    ) -> Result<GitObject, CommandError> {
        let mut objects = HashMap::<String, GitObject>::new();

        while let Ok(mode) = Mode::read_from(stream) {
            logger.log(&format!("mode: {:?}", mode));
            let type_src = {
                let mut type_buf = [0; 1];
                stream
                    .read_exact(&mut type_buf)
                    .map_err(|error| CommandError::InvalidMode)?;
                match type_buf {
                    [0] => "blob",
                    [1] => "tree",
                    [2] => "commit",
                    [3] => "tag",
                    _ => return Err(CommandError::ObjectTypeError),
                }
            };
            let mut hash = vec![0; 20];
            stream
                .read_exact(&mut hash)
                .map_err(|error| CommandError::ObjectHashNotKnown)?;
            let hash_str = hash
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<Vec<_>>()
                .join("");

            let mut size_be = [0; 4];
            stream
                .read_exact(&mut size_be)
                .map_err(|error| CommandError::FailToCalculateObjectSize)?;
            let size = u32::from_be_bytes(size_be) as usize;
            let mut name = vec![0; size];
            stream
                .read_exact(&mut name)
                .map_err(|error| CommandError::FailToOpenSatginArea(error.to_string()))?;

            logger.log(&format!("objects_database::read : {:?}", hash_str));
            let object = objects_database::read_object(&hash_str, logger)?;
            logger.log(&format!("Success! : {:?}", object));
            objects.insert(hash_str, object);
        }
        Ok(Box::new(Self {
            path: path.to_string(),
            objects,
        }))
    }

    pub(crate) fn display_from_hash(
        stream: &mut dyn Read,
        len: usize,
        path: String,
        hash: &str,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let mut objects = Vec::<(Mode, String, String, String)>::new();

        while let Ok(mode) = Mode::read_from(stream) {
            logger.log(&format!("mode: {:?}", mode));
            let type_src = {
                let mut type_buf = [0; 1];
                stream
                    .read_exact(&mut type_buf)
                    .map_err(|error| CommandError::InvalidMode)?;
                match type_buf {
                    [0] => "blob",
                    [1] => "tree",
                    [2] => "commit",
                    [3] => "tag",
                    _ => return Err(CommandError::ObjectTypeError),
                }
            };
            let mut hash = vec![0; 20];
            stream
                .read_exact(&mut hash)
                .map_err(|error| CommandError::ObjectHashNotKnown)?;
            let hash_str = hash
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<Vec<_>>()
                .join("");

            let mut size_be = [0; 4];
            stream
                .read_exact(&mut size_be)
                .map_err(|_| CommandError::FailToCalculateObjectSize)?;
            let size = u32::from_be_bytes(size_be) as usize;
            let mut name = vec![0; size];
            stream
                .read_exact(&mut name)
                .map_err(|error| CommandError::FailToOpenSatginArea(error.to_string()))?;

            logger.log(&format!("objects_database::read : {:?}", hash_str));
            let object = objects_database::read_object(&hash_str, logger)?;
            logger.log(&format!("Success! : {:?}", object));
            let name_str = String::from_utf8(name).map_err(|_| CommandError::FileNameError)?;
            objects.push((mode, type_src.to_string(), hash_str, name_str));
        }
        for (mode, type_str, hash, name) in objects {
            writeln!(output, "{} {} {}    {}", mode, type_str, hash, name)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }

}

impl GitObjectTrait for Tree {
    fn as_mut_tree(&mut self) -> Option<&mut Tree> {
        Some(self)
    }

    fn clone_object(&self) -> GitObject {
        Box::new(self.clone())
    }

    fn type_str(&self) -> String {
        "tree".to_string()
    }

    fn content(&self) -> Result<Vec<u8>, CommandError> {
        let mut content = Vec::new();
        for (path, object) in self.objects.iter() {
            let hash_str = objects_database::write(object.to_owned())?;
            let filename = get_name(path)?;
            let type_byte = type_byte(&self.type_str())?;
            object.mode().write_to(&mut content)?;
            content.extend_from_slice(&[type_byte]);
            let hash_hex = hex_string_to_u8_vec(&hash_str);
            content.extend_from_slice(&hash_hex);
            let size_be = (filename.len() as u32).to_be_bytes();
            content.extend_from_slice(&size_be);
            content.extend_from_slice(filename.as_bytes());
        }
        Ok(content)
    }

    fn add_path(
        &mut self,
        vector_path: Vec<&str>,
        current_depth: usize,
        hash: &String,
    ) -> Result<(), CommandError> {
        let current_path_str = vector_path[..current_depth + 1].join("/");

        let aux = vector_path.len()-1;
        if current_depth != aux {
            self.add_tree(&current_path_str, vector_path, current_depth, hash)?;
        } else {
            self.add_blob(&current_path_str, hash)?;
        }
        Ok(())
    }

    fn mode(&self) -> Mode {
        Mode::Tree
    }

    fn to_string_priv(&self) -> String {
        "ASDF".to_string()
        // format!(
        //     "{} {} {:?}    {:?}\n",
        //     self.mode(),
        //     self.type_str(),
        //     self.hash(),
        //     self.filename()
        // )
    }

    // Obtiene el nombre de un archivo dada su ruta. Si la ruta no existe, devuelve error.
    // pub fn get_name(&s)-> Result<String, CommandError>{

    //     let path = Path::new(path);
    //     if !path.exists(){
    //         return Err(CommandError::FileNotFound(path.to_string()));
    //     }
    //     if let Some(file_name) = path.file_name() {
    //         if let Some(name_str) = file_name.to_str() {
    //             return Ok(name_str.to_string());
    //         }
    //     }
    //     Err(CommandError::FileNotFound(path.to_owned()))
    // }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string_priv())
    }
}

fn type_byte(type_str: &str) -> Result<u8, CommandError> {
    match type_str {
        "blob" => Ok(0),
        "tree" => Ok(1),
        "commit" => Ok(2),
        "tag" => Ok(3),
        _ => return Err(CommandError::ObjectTypeError),
    }
}

fn hex_string_to_u8_vec(hex_string: &str) -> [u8; 20] {
    let mut result = [0; 20];
    let mut chars = hex_string.chars().peekable();

    let mut i = 0;
    while let Some(c1) = chars.next() {
        if let Some(c2) = chars.peek() {
            if let (Some(n1), Some(n2)) = (c1.to_digit(16), c2.to_digit(16)) {
                result[i] = (n1 * 16 + n2) as u8;
                chars.next();
                i += 1;
            } else {
                panic!("Invalid hex string");
            }
        } else {
            panic!("Invalisd hex string");
        }
    }

    result
}




#[cfg(test)]
mod tests {
    use std::{
        env,
        fs::{create_dir_all, File},
    };

    use chrono::format::format;

    use super::*;

    #[test]
    fn given_a_path_a_tree_is_created() {
        let path = "test".to_string();
        let tree = Tree::new(path.clone());
        assert_eq!(tree.path, path);
    }

    #[test]
    fn given_a_path_a_tree_is_created_with_empty_objects() {
        let path = "test".to_string();
        let tree = Tree::new(path.clone());
        assert_eq!(tree.objects.len(), 0);
    }

    #[test]
    fn given_a_path_a_tree_is_created_with_empty_objects_and_then_add_a_blob() {
        let file_name = "test.txt".to_string();
        let current_dir = env::current_dir().unwrap();
        println!("The current directory is {}", current_dir.display());

        // Abre el archivo en modo escritura (se creará si no existe)
        let mut file = File::create(&file_name).expect("No se pudo crear el archivo");

        // Contenido que deseas escribir en el archivo
        let content = "test";

        // Escribe el contenido en el archivo
        match file.write_all(content.as_bytes()) {
            Ok(_) => println!("Archivo creado y contenido escrito con éxito."),
            Err(err) => eprintln!("Error al escribir en el archivo: {}", err),
        }

        let path = format!("{}/{}", current_dir.display(), file_name);
        let mut tree = Tree::new(path.clone());
        let path_name = "testfile.txt".to_string();
        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        tree.add_blob(&path, &hash).unwrap();
        assert_eq!(tree.objects.len(), 1);

        // borramos el archivo
        let _ = std::fs::remove_file(file_name);
    }

    #[test]
    fn hhh() {
        let files = [
            "dir0/dir1/dir2/meli.txt".to_string(),
            "dir0/dir1/ian.txt".to_string(),
            "dir0/pato.txt".to_string(),
            "sofi.txt".to_string(),
        ];
        create_dir_all("dir0/dir1/dir2/").unwrap();
        let mut tree = Tree::new("".to_string());

        // Creamos los files
        for file_str in files.iter() {
            // let file_name = format!("{}/{}", "dir_padre".to_string(), file_str);
            let content = "test";

            // Escribe el contenido en el archivo
            let mut file = File::create(&file_str).unwrap();

            file.write_all(content.as_bytes()).unwrap();
        }

        let hash = "30d74d258442c7c65512eafab474568dd706c430".to_string();
        for path in files {
            let vector_path = path.split("/").collect::<Vec<_>>();
            let current_depth: usize = 0;
            _ = tree.add_path_tree(vector_path, current_depth, &hash);
        }

        let _ = std::fs::remove_dir_all("dir_padre");
    }
}
